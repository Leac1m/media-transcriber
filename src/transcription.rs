use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// A loaded Whisper model. Loading is the slow part, so load once and reuse
/// it for every file.
pub struct Transcriber {
    ctx: WhisperContext,
}

impl Transcriber {
    pub fn load(model_path: &str) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("Failed to load model: {}", e))?;
        Ok(Self { ctx })
    }

    /// Returns `(text_with_timestamps, text_without_timestamps)`.
    pub fn transcribe<F>(
        &self,
        audio_data: &[f32],
        progress_callback: F,
    ) -> Result<(String, String), String>
    where
        F: FnMut(i32) + Send + 'static,
    {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|_| "Failed to create state".to_string())?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // whisper.cpp defaults to English; detect the spoken language instead.
        params.set_language(Some("auto"));

        params.set_progress_callback_safe(progress_callback);

        state
            .full(params, audio_data)
            .map_err(|e| format!("Failed to transcribe: {}", e))?;

        let num_segments = state.full_n_segments();
        let mut full_text_stamped = String::new();
        let mut full_text_raw = String::new();

        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let text = segment.to_str().unwrap_or_else(|_| "".into());

                let start = segment.start_timestamp();
                let start_sec = start / 100;
                let m = start_sec / 60;
                let s = start_sec % 60;
                let ms = (start % 100) * 10;

                let stamped_line = format!("[{:02}:{:02}.{:03}] {}\n", m, s, ms, text.trim());

                full_text_stamped.push_str(&stamped_line);
                full_text_raw.push_str(&text);
                full_text_raw.push('\n');
            }
        }

        if full_text_raw.trim().is_empty() {
            full_text_raw = "No speech detected.".to_string();
            full_text_stamped = full_text_raw.clone();
        }

        Ok((full_text_stamped, full_text_raw))
    }
}
