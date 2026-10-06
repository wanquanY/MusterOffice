//! Disposable decoder or finite encoder. Never opens media or output paths.
mod encoding;
mod indexing;
mod wire;
use mo_media::{ErrorKind, MediaError};
use mo_media_ffmpeg::{Container, Decoder};
use mo_media_wire::{Command, Event, PROFILE, PlaneLayout, Request, validate_limits};
use mo_native_limits::apply_to_current_worker;
use std::{cell::RefCell, io, rc::Rc};
use wire::Wire;

enum State {
    Empty,
    Indexed(indexing::Indexed),
    Open(Container<'static>),
    Selected(Decoder<'static>),
    Encoding(encoding::Encoding),
    Failed,
    Finished,
    Closed,
}
fn state_error() -> MediaError {
    MediaError::new(
        ErrorKind::State,
        "command not permitted in current worker state",
    )
}
fn run() -> io::Result<()> {
    let io = Rc::new(RefCell::new(Wire::new()));
    let mut state = State::Empty;
    let mut last_call = 0u64;
    loop {
        let request: Option<Request> = mo_media_wire::read_header(&mut io.borrow_mut().input)?;
        let Some(request) = request else {
            return if matches!(state, State::Closed) {
                Ok(())
            } else {
                Err(io::Error::other("EOF before close"))
            };
        };
        if matches!(state, State::Closed) || Some(request.call_id) != last_call.checked_add(1) {
            return Err(io::Error::other(
                "unexpected media call id or command after close",
            ));
        }
        last_call = request.call_id;
        io.borrow_mut().call_id = last_call;
        let result = match request.command {
            Command::DescribeVideoTime { track } => match &mut state {
                State::Indexed(index) => index.timing(track).map(|timing| Event::VideoTiming {
                    summary: timing.summary().clone(),
                }),
                _ => Err(state_error()),
            },
            Command::SelectVideoTime { track, time, side } => match &mut state {
                State::Indexed(index) => index
                    .timing(track)
                    .and_then(|timing| {
                        timing
                            .select(&time, side, &|| false)
                            .map_err(indexing::timing_error)
                    })
                    .map(|selection| Event::VideoTimeSelected { selection }),
                _ => Err(state_error()),
            },
            Command::OpenIndex {
                profile,
                byte_length,
                limits,
                process_limits,
            } if matches!(state, State::Empty) => {
                if profile != PROFILE {
                    return Err(io::Error::other("media worker protocol mismatch"));
                }
                limits.validate().map_err(io::Error::other)?;
                let applied = apply_to_current_worker(process_limits).map_err(io::Error::other)?;
                match indexing::open(Rc::clone(&io), byte_length, limits) {
                    Ok(index) => {
                        let summary = mo_media_wire::IndexSummary::from_index(&index.raw);
                        state = State::Indexed(index);
                        Ok(Event::Indexed {
                            profile: PROFILE.into(),
                            summary,
                            applied,
                        })
                    }
                    Err(error) => Err(error),
                }
            }
            Command::QueryIndex { query } => match &state {
                State::Indexed(index) => query
                    .execute(&index.raw)
                    .map(|page| Event::IndexPage { page })
                    .map_err(|error| MediaError::new(ErrorKind::Invalid, error.to_string())),
                _ => Err(state_error()),
            },
            Command::OpenEncoder {
                profile,
                configuration,
                limits,
                process_limits,
            } if matches!(state, State::Empty) => {
                if profile != PROFILE {
                    return Err(io::Error::other("media worker protocol mismatch"));
                }
                mo_media_wire::encode_video_lengths(configuration, limits)?;
                let applied = apply_to_current_worker(process_limits).map_err(io::Error::other)?;
                match encoding::Encoding::open(Rc::clone(&io), configuration, limits) {
                    Ok(encoder) => {
                        state = State::Encoding(encoder);
                        Ok(Event::EncoderOpened {
                            profile: PROFILE.into(),
                            configuration,
                            applied,
                        })
                    }
                    Err(error) => Err(error),
                }
            }
            Command::EncodeVideo { pts, duration } => match &mut state {
                State::Encoding(encoder) => encoder.video(&io, pts, duration),
                _ => Err(state_error()),
            },
            Command::EncodeAudio {
                first_sample,
                frames,
            } => match &mut state {
                State::Encoding(encoder) => encoder.audio(&io, first_sample, frames),
                _ => Err(state_error()),
            },
            Command::FinishEncoder => match std::mem::replace(&mut state, State::Failed) {
                State::Encoding(mut encoder) => {
                    let result = encoder.finish();
                    if result.is_ok() {
                        state = State::Finished;
                    }
                    result
                }
                _ => Err(state_error()),
            },
            Command::Open {
                profile,
                timeline,
                byte_length,
                limits,
                process_limits,
            } if matches!(state, State::Empty) => {
                if profile != PROFILE {
                    return Err(io::Error::other("media worker protocol mismatch"));
                }
                validate_limits(limits, byte_length)?;
                // A failed installation can have partially changed process state.
                // Exit immediately; never continue decoding without the limits.
                let applied = apply_to_current_worker(process_limits).map_err(io::Error::other)?;
                let resource_io = Rc::clone(&io);
                let input = move |offset, destination: &mut [u8]| {
                    resource_io.borrow_mut().read_source(offset, destination)
                };
                match Container::open_with_timeline(input, byte_length, limits, timeline, || false)
                {
                    Ok(container) => {
                        let streams = container.streams().to_vec();
                        state = State::Open(container);
                        Ok(Event::Opened {
                            profile: PROFILE.into(),
                            timeline,
                            streams,
                            applied,
                        })
                    }
                    Err(error) => Err(error),
                }
            }
            Command::Select { index } => match std::mem::replace(&mut state, State::Failed) {
                State::Open(container) => match container.select(index) {
                    Ok(decoder) => {
                        state = State::Selected(decoder);
                        Ok(Event::Selected)
                    }
                    Err(error) => Err(error),
                },
                _ => Err(state_error()),
            },
            Command::Next => {
                if let State::Selected(decoder) = &mut state {
                    match decoder.next_frame() {
                        Ok(Some(frame)) => {
                            let planes = frame
                                .planes
                                .iter()
                                .map(|p| PlaneLayout {
                                    row_bytes: p.row_bytes as u64,
                                    rows: p.rows as u64,
                                })
                                .collect();
                            let event = Event::Frame {
                                info: frame.info,
                                planes,
                            };
                            io.borrow_mut()
                                .respond(&event, frame.planes.iter().map(|p| p.bytes.as_slice()))?;
                            continue;
                        }
                        Ok(None) => Ok(Event::Eof),
                        Err(error) => Err(error),
                    }
                } else {
                    Err(state_error())
                }
            }
            Command::Seek { ticks } => {
                if let State::Selected(decoder) = &mut state {
                    decoder.seek(ticks).map(|_| Event::Seeked)
                } else {
                    Err(state_error())
                }
            }
            Command::Close => {
                state = State::Closed;
                Ok(Event::Closed)
            }
            _ => Err(state_error()),
        };
        let event = match result {
            Ok(event) => event,
            Err(error) => {
                state = State::Failed;
                Event::Error { error }
            }
        };
        io.borrow_mut().respond(&event, std::iter::empty())?;
        // Remain alive after Closed/Error until owner closes input or kills us.
        // This avoids racing a final response against process exit in the pipe.
    }
}
fn main() {
    if std::env::args_os().len() != 1 {
        eprintln!("media worker takes no arguments");
        std::process::exit(2);
    }
    if let Err(error) = run() {
        eprintln!("media worker: {error}");
        std::process::exit(1);
    }
}
