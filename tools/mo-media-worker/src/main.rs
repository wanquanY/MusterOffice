//! Disposable single-source/single-stream decoder. Never opens media paths.
mod wire;
use mo_media::{ErrorKind, MediaError};
use mo_media_ffmpeg::{Container, Decoder};
use mo_media_wire::{Command, Event, PROFILE, PlaneLayout, Request, validate_limits};
use mo_native_limits::apply_to_current_worker;
use std::{cell::RefCell, io, rc::Rc};
use wire::Wire;

enum State {
    Empty,
    Open(Container<'static>),
    Selected(Decoder<'static>),
    Failed,
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
            Command::Open {
                profile,
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
                match Container::open(input, byte_length, limits, || false) {
                    Ok(container) => {
                        let streams = container.streams().to_vec();
                        state = State::Open(container);
                        Ok(Event::Opened {
                            profile: PROFILE.into(),
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
