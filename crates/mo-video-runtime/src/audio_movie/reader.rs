use super::*;
impl<S: AudioResources> AudioResources for MovieAudioResource<S> {
    fn describe(&mut self, binding: &MediaBinding) -> Result<AudioAsset> {
        if *binding != self.binding {
            return Err(AudioError::Invalid("changed movie audio binding"));
        }
        Ok(self.plan.asset.clone())
    }
    fn read(
        &mut self,
        binding: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        active(cancelled)?;
        if *binding != self.binding || *asset != self.plan.asset {
            return Err(AudioError::Invalid("changed movie audio binding"));
        }
        if range.first < 0 || range.end < range.first || range.end > asset.available.end {
            return Err(AudioError::Invalid("movie PCM read range"));
        }
        let frames = usize::try_from(range.end - range.first).map_err(|_| AudioError::Budget)?;
        let channels = asset.layout.len();
        let output_bytes = frames
            .checked_mul(channels)
            .and_then(|n| n.checked_mul(4))
            .ok_or(AudioError::Budget)?;
        let plan = &self.plan;
        if frames > plan.limits.max_block_frames
            || frames > limits.max_frames
            || output_bytes > limits.max_bytes
            || output_bytes > plan.limits.max_working_bytes
        {
            return Err(AudioError::Budget);
        }
        if output_bytes
            .checked_add(self.retained_filter_bytes())
            .is_none_or(|n| n > self.plan.limits.max_working_bytes)
        {
            self.filter = None;
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(frames * channels)
            .map_err(|_| AudioError::Budget)?;
        output.resize(frames * channels, 0.);
        let mut work = 0usize;
        let first_segment = plan.segments.partition_point(|s| s.end <= range.first);
        for segment in &plan.segments[first_segment..] {
            if segment.first >= range.end {
                break;
            }
            active(cancelled)?;
            let first = range.first.max(segment.first);
            let end = range.end.min(segment.end);
            let Some(source) = &segment.source else {
                continue;
            };
            let count = (end - first) as usize;
            let start = source.first.add(
                &source
                    .step
                    .mul(&R::integer(first - segment.first), plan.limits.exact_bits)?,
                plan.limits.exact_bits,
            )?;
            if self
                .filter
                .as_ref()
                .is_none_or(|(step, _)| step.compare(&source.step).is_ne())
            {
                // Release the old filter before allocating another rate profile.
                self.filter = None;
                let kernel =
                    SincResampler::estimate_kernel_bytes(&source.step, plan.limits.exact_bits)?;
                if kernel
                    .checked_add(output_bytes)
                    .is_none_or(|n| n > plan.limits.max_working_bytes)
                {
                    return Err(AudioError::Budget);
                }
                self.filter = Some((
                    source.step.clone(),
                    SincResampler::new(
                        &source.step,
                        plan.limits.exact_bits,
                        plan.limits.resample,
                        cancelled,
                    )?,
                ));
            }
            let filter = &self
                .filter
                .as_ref()
                .ok_or(AudioError::Invalid("missing movie audio filter"))?
                .1;
            work = work
                .checked_add(filter.multiply_adds(&start, count, channels)?)
                .ok_or(AudioError::Budget)?;
            if work > plan.limits.resample.max_multiply_adds {
                return Err(AudioError::Budget);
            }
            let needed = filter
                .required_range(&start, count)?
                .intersection(source.available);
            let source_frames =
                usize::try_from(needed.end - needed.first).map_err(|_| AudioError::Budget)?;
            let source_bytes = source_frames
                .checked_mul(channels)
                .and_then(|n| n.checked_mul(4))
                .ok_or(AudioError::Budget)?;
            let processed_bytes = count * channels * 4;
            let total = output_bytes
                .checked_add(source_bytes)
                .and_then(|n| n.checked_add(processed_bytes))
                .and_then(|n| n.checked_add(filter.kernel_bytes()))
                .ok_or(AudioError::Budget)?;
            if total > plan.limits.max_working_bytes
                || source_frames > plan.limits.resample.pcm.max_frames
                || source_bytes > plan.limits.resample.pcm.max_bytes
            {
                return Err(AudioError::Budget);
            }
            if self.raw.describe(&plan.raw_binding)? != plan.raw_asset {
                return Err(AudioError::Invalid("changed raw PCM capability"));
            }
            let raw = if source_frames == 0 {
                PcmBlock::new(
                    plan.raw_asset.sample_rate,
                    plan.raw_asset.layout.clone(),
                    needed.first,
                    vec![],
                    plan.limits.resample.pcm,
                    cancelled,
                )?
            } else {
                self.raw.read(
                    &plan.raw_binding,
                    &plan.raw_asset,
                    needed,
                    PcmLimits {
                        max_frames: source_frames,
                        max_bytes: source_bytes,
                    },
                    cancelled,
                )?
            };
            active(cancelled)?;
            if raw.sample_rate() != plan.raw_asset.sample_rate
                || raw.layout() != &plan.raw_asset.layout
                || raw.first_frame() != needed.first
                || raw.end_frame() != needed.end
            {
                return Err(AudioError::Invalid("raw PCM returned wrong footprint"));
            }
            let processed = filter.process(
                &raw,
                source.available,
                &start,
                asset.sample_rate,
                first,
                count,
                plan.limits.resample,
                cancelled,
            )?;
            let offset = (first - range.first) as usize * channels;
            output[offset..offset + count * channels].copy_from_slice(processed.samples());
        }
        PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            range.first,
            output,
            limits,
            cancelled,
        )
    }
}
