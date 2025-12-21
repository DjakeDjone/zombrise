use crate::music_generation::Song;
use rodio::{OutputStream, Sink, Source};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct MusicPlayer {
    synthesizer: Arc<Mutex<Synthesizer>>,
    _stream: OutputStream,
    stream_handle: rodio::OutputStreamHandle,
    current_sink: Option<Sink>,
}

impl MusicPlayer {
    pub fn new(sf2_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(sf2_path)?;
        let sound_font = Arc::new(SoundFont::new(&mut file)?);
        let settings = SynthesizerSettings::new(44100);
        let synthesizer = Synthesizer::new(&sound_font, &settings)?;

        let (_stream, stream_handle) = OutputStream::try_default()?;

        Ok(Self {
            synthesizer: Arc::new(Mutex::new(synthesizer)),
            _stream,
            stream_handle,
            current_sink: None,
        })
    }

    pub fn play_song(&mut self, song: &Song) {
        // Stop current song if playing
        if let Some(sink) = self.current_sink.take() {
            sink.stop();
        }

        let sink = Sink::try_new(&self.stream_handle).unwrap();

        // Convert song to a simpler internal representation or just process it directly
        // For simplicity, let's just render the melody for now.
        // A real implementation would handle polyphony and timing more carefully.

        let sample_rate = 44100;

        // We'll create a custom Source that feeds data from the synthesizer
        let source = SynthSource::new(self.synthesizer.clone(), song, sample_rate);

        sink.append(source);
        // sink.sleep_until_end(); // Blocking call removed
        self.current_sink = Some(sink);
    }
}

// Cloneable wrapper to pass to the Source
// Cloneable wrapper to pass to the Source
#[derive(Clone)]
struct SynthSource {
    synthesizer: Arc<Mutex<Synthesizer>>,
    events: Vec<(f32, MidiEvent)>, // (time_seconds, event)
    current_time: f32,
    sample_rate: u32,
    current_event_index: usize,
    buffer: Vec<f32>,
    buffer_pos: usize,
}

#[derive(Clone, Copy, Debug)]
enum MidiEvent {
    NoteOn { midi: i32, velocity: u8 },
    NoteOff { midi: i32 },
}

impl SynthSource {
    fn new(synthesizer: Arc<Mutex<Synthesizer>>, song: &Song, sample_rate: u32) -> Self {
        let mut events = Vec::new();
        let beat_duration = 60.0 / song.tempo;

        for line in &song.lines {
            let mut current_time = 0.0;
            for note in &line.notes {
                if let Some(n) = note.note {
                    let duration_secs = note.duration * beat_duration;

                    // Note On
                    events.push((
                        current_time,
                        MidiEvent::NoteOn {
                            midi: n.to_midi(),
                            velocity: note.velocity,
                        },
                    ));

                    // Note Off (slightly shorter than full duration to articulate)
                    events.push((
                        current_time + duration_secs * 0.95,
                        MidiEvent::NoteOff { midi: n.to_midi() },
                    ));

                    current_time += duration_secs;
                } else {
                    // Rest
                    current_time += note.duration * beat_duration;
                }
            }
        }

        // Sort events by time
        events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        Self {
            synthesizer,
            events,
            current_time: 0.0,
            sample_rate,
            current_event_index: 0,
            buffer: Vec::new(),
            buffer_pos: 0,
        }
    }
}

impl Iterator for SynthSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer_pos >= self.buffer.len() {
            // Refill buffer
            let mut left = vec![0.0f32; 1024];
            let mut right = vec![0.0f32; 1024];

            // Check for new events
            let samples_per_chunk = left.len();
            let time_step = samples_per_chunk as f32 / self.sample_rate as f32;
            let end_time = self.current_time + time_step;

            while self.current_event_index < self.events.len() {
                let (event_time, event) = self.events[self.current_event_index];
                if event_time < end_time {
                    // Process event
                    let mut synth = self.synthesizer.lock().unwrap();
                    match event {
                        MidiEvent::NoteOn { midi, velocity } => {
                            synth.note_on(0, midi, velocity as i32);
                        }
                        MidiEvent::NoteOff { midi } => {
                            synth.note_off(0, midi);
                        }
                    }
                    self.current_event_index += 1;
                } else {
                    break;
                }
            }

            let mut synth = self.synthesizer.lock().unwrap();
            synth.render(&mut left, &mut right);
            self.current_time += time_step;

            // Interleave locally for the iterator
            self.buffer.clear();
            for i in 0..left.len() {
                self.buffer.push((left[i] + right[i]) * 0.5);
            }
            self.buffer_pos = 0;
        }

        if self.buffer_pos < self.buffer.len() {
            let sample = self.buffer[self.buffer_pos];
            self.buffer_pos += 1;
            Some(sample)
        } else {
            // Silence if song is over but we keep stream open
            Some(0.0)
        }
    }
}

impl Source for SynthSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
