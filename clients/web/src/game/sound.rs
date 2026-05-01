//! Synthesised sound effects using the Web Audio API.
//!
//! All public functions are no-ops on non-WASM targets so callers need no
//! `#[cfg]` guards themselves.

#[cfg(target_arch = "wasm32")]
mod inner {
    use std::cell::RefCell;
    use web_sys::{AudioContext, OscillatorType};

    thread_local! {
        static CTX: RefCell<Option<AudioContext>> = const { RefCell::new(None) };
    }

    fn with_ctx<F: FnOnce(&AudioContext)>(f: F) {
        CTX.with(|cell| {
            let mut opt = cell.borrow_mut();
            if opt.is_none() {
                *opt = AudioContext::new().ok();
            }
            if let Some(ctx) = opt.as_ref() {
                f(ctx);
            }
        });
    }

    /// Schedule a single oscillator tone with an exponential gain decay.
    ///
    /// - `start_offset`: seconds from `ctx.current_time()` when the tone starts
    /// - `duration`: how long (in seconds) until gain reaches ~0
    fn play_tone(
        ctx: &AudioContext,
        freq: f32,
        gain: f32,
        duration: f64,
        start_offset: f64,
        wave: OscillatorType,
    ) {
        let t0 = ctx.current_time() + start_offset;
        let t1 = t0 + duration;

        let Ok(osc) = ctx.create_oscillator() else {
            return;
        };
        let Ok(gain_node) = ctx.create_gain() else {
            return;
        };

        osc.set_type(wave);
        osc.frequency().set_value(freq);

        let gain_param = gain_node.gain();
        let _ = gain_param.set_value_at_time(gain, t0);
        // exponential_ramp requires a positive target; 0.001 is inaudible
        let _ = gain_param.exponential_ramp_to_value_at_time(0.001, t1);

        let dest = ctx.destination();
        let _ = osc.connect_with_audio_node(&gain_node);
        let _ = gain_node.connect_with_audio_node(&dest);

        let _ = osc.start_with_when(t0);
        let _ = osc.stop_with_when(t1);
    }

    /// Short wooden clack: sine fundamental + triangle body resonance, ~80 ms.
    pub fn play_checker_move() {
        with_ctx(|ctx| {
            // Sine at 300 Hz for the clean attack click
            play_tone(ctx, 300.0, 0.55, 0.080, 0.000, OscillatorType::Sine);
            // Triangle at 150 Hz for the woody body resonance
            play_tone(ctx, 150.0, 0.35, 0.070, 0.005, OscillatorType::Triangle);
            // Sub at 80 Hz for weight
            play_tone(ctx, 80.0, 0.20, 0.060, 0.008, OscillatorType::Triangle);
        });
    }

    /// Cinematic dice roll: ~500 ms of rolling texture + 5 impact transients.
    ///
    /// Two layers:
    /// - A dense series of detuned sawtooth bursts that thin out over time,
    ///   modelling the continuous scrape/rattle of dice tumbling.
    /// - Five percussive impacts (square clicks + triangle thuds) whose
    ///   inter-arrival gap shrinks as the dice decelerate and settle.
    pub fn play_dice_roll_cinematic() {
        with_ctx(|ctx| {
            // ── Continuous rolling texture ─────────────────────────────────
            // 16 steps over 440 ms; each step is two detuned sawtooth waves
            // (the interference between them produces a noise-like texture).
            // Gain fades by ~55 % from first to last step.
            const N: u32 = 16;
            for i in 0..N {
                let t = i as f64 * 0.028;
                let g = 0.017 * (1.0 - i as f32 / N as f32 * 0.55);
                // Quasi-random frequencies so each step sounds different.
                let f1 = 310.0 + (i as f32 * 29.3 % 280.0);
                let f2 = 480.0 + (i as f32 * 43.7 % 220.0);
                play_tone(ctx, f1, g, 0.028, t, OscillatorType::Sawtooth);
                play_tone(ctx, f2, g * 0.70, 0.028, t, OscillatorType::Sawtooth);
            }

            // ── Impact transients ──────────────────────────────────────────
            // Gaps narrow toward the end (0.13 → 0.11 → 0.10 → 0.08 s),
            // mimicking dice decelerating and settling.
            let impacts: &[(f64, f32)] = &[(0.00, 1.00), (0.13, 0.8), (0.24, 0.54), (0.34, 0.30)];
            for &(t_off, amp) in impacts {
                // Hard click: bright square partials → percussive attack
                for &freq in &[700.0f32, 1_050.0, 1_500.0] {
                    play_tone(ctx, freq, amp * 0.03, 0.022, t_off, OscillatorType::Square);
                }
                // Woody body thud: two low triangle partials
                play_tone(
                    ctx,
                    130.0,
                    amp * 0.05,
                    0.070,
                    t_off,
                    OscillatorType::Triangle,
                );
                play_tone(
                    ctx,
                    68.0,
                    amp * 0.07,
                    0.090,
                    t_off,
                    OscillatorType::Triangle,
                );
            }
        });
    }

    /// Play the pre-recorded dice-roll MP3 asset.
    pub fn play_dice_roll() {
        if let Ok(audio) = web_sys::HtmlAudioElement::new_with_src("/diceroll.mp3") {
            audio.set_volume(0.2);
            let _ = audio.play();
        }
    }

    /// Ascending three-note chime (C5 – E5 – G5).
    pub fn play_points_scored() {
        with_ctx(|ctx| {
            let notes: [(f32, f64); 3] = [(523.25, 0.0), (659.25, 0.14), (783.99, 0.28)];
            for (freq, offset) in notes {
                play_tone(ctx, freq, 0.28, 0.30, offset, OscillatorType::Sine);
            }
        });
    }

    /// Brief high tick for the jackpot-style points counter (one call per increment).
    pub fn play_points_tick() {
        with_ctx(|ctx| {
            play_tone(ctx, 880.0, 0.18, 0.055, 0.000, OscillatorType::Sine);
            play_tone(ctx, 1320.0, 0.07, 0.035, 0.000, OscillatorType::Sine);
        });
    }

    /// Brief low tick for the jackpot-style points counter (one call per increment).
    pub fn play_opp_points_tick() {
        with_ctx(|ctx| {
            play_tone(ctx, 680.0, 0.18, 0.055, 0.000, OscillatorType::Sine);
            play_tone(ctx, 1020.0, 0.07, 0.035, 0.000, OscillatorType::Sine);
        });
    }

    /// Triumphant four-note fanfare (C5 – E5 – G5 – C6).
    pub fn play_hole_scored() {
        with_ctx(|ctx| {
            let notes: [(f32, f64, f64); 4] = [
                (523.25, 0.0, 0.35),
                (659.25, 0.17, 0.35),
                (783.99, 0.34, 0.35),
                (1046.5, 0.51, 0.55),
            ];
            for (freq, offset, dur) in notes {
                play_tone(ctx, freq, 0.12, dur, offset, OscillatorType::Sine);
            }
        });
    }
}

// ── Public API: WASM delegates to `inner`, other targets are no-ops ───────────

#[cfg(target_arch = "wasm32")]
pub use inner::{
    play_checker_move, play_dice_roll, play_dice_roll_cinematic, play_hole_scored,
    play_opp_points_tick, play_points_scored, play_points_tick,
};

#[cfg(not(target_arch = "wasm32"))]
pub fn play_checker_move() {}
#[cfg(not(target_arch = "wasm32"))]
pub fn play_dice_roll() {}
#[cfg(not(target_arch = "wasm32"))]
pub fn play_dice_roll_cinematic() {}
#[cfg(not(target_arch = "wasm32"))]
pub fn play_points_scored() {}
#[cfg(not(target_arch = "wasm32"))]
pub fn play_points_tick() {}
#[cfg(not(target_arch = "wasm32"))]
pub fn play_opp_points_tick() {}
#[cfg(not(target_arch = "wasm32"))]
pub fn play_hole_scored() {}
