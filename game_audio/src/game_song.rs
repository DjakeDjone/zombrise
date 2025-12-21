use crate::music_generation::{Chord, Line, Note, NoteType, Scale, ScaleMotif, Song, TimedNote};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intensity {
    Calm,
    Tension,
    Combat,
}

/// Chord progression for the song - returns chords for each measure
fn get_chord_progression(
    intensity: Intensity,
    scale: &Scale,
    octave: u8,
    num_measures: usize,
) -> Vec<Chord> {
    let mut chords = Vec::with_capacity(num_measures);

    // Define chord degrees for each intensity
    let progression = match intensity {
        // i - VI - III - VII (Am - F - C - G in A minor)
        Intensity::Calm => vec![0, 5, 2, 6, 0, 5, 4, 0],
        // i - bII - v - i (dark Phrygian sound)
        Intensity::Tension => vec![0, 1, 4, 0, 0, 1, 6, 0],
        // i - VII - VI - VII (driving, aggressive)
        Intensity::Combat => vec![0, 6, 5, 6, 0, 6, 5, 4],
    };

    for i in 0..num_measures {
        let degree = progression[i % progression.len()];
        let root_note = Note::new(scale.notes[degree], octave);
        let chord = Chord::from_scale(&Scale::from_type(root_note, scale_type_from_degree(degree)));
        chords.push(chord);
    }
    chords
}

/// Determine scale type based on chord degree (simplified)
fn scale_type_from_degree(degree: usize) -> crate::music_generation::ScaleType {
    use crate::music_generation::ScaleType;
    match degree {
        0 | 3 | 4 => ScaleType::Minor,
        _ => ScaleType::Major,
    }
}

/// Generate a walking bass line for calm intensity
fn generate_walking_bass(chord: &Chord, scale: &Scale, octave: u8) -> Vec<TimedNote> {
    let mut notes = Vec::new();
    let root = chord.root.with_octave(octave);

    // Walking bass: root, approach, fifth, approach back
    notes.push(TimedNote::new(root, 1.0, 75));

    // Walk up to the third
    let third_idx = 2;
    let third = Note::new(scale.notes[third_idx % scale.notes.len()], octave);
    notes.push(TimedNote::new(third, 1.0, 70));

    // Hit the fifth
    let fifth = Note::new(scale.notes[4 % scale.notes.len()], octave);
    notes.push(TimedNote::new(fifth, 1.0, 72));

    // Approach note back to root (chromatic or diatonic)
    if rand::random::<bool>() {
        // Chromatic approach from below
        let approach_note = Note::new(scale.notes[6 % scale.notes.len()], octave);
        notes.push(TimedNote::new(approach_note, 1.0, 68));
    } else {
        // Fifth again
        notes.push(TimedNote::new(fifth, 0.5, 65));
        notes.push(TimedNote::pause(0.5));
    }

    notes
}

/// Generate an ostinato bass pattern for tension
fn generate_ostinato_bass(chord: &Chord, octave: u8, measure: usize) -> Vec<TimedNote> {
    let mut notes = Vec::new();
    let root = chord.root.with_octave(octave);

    // Pulsing ostinato with occasional variation
    let pattern = if measure % 4 == 3 {
        // Variation every 4th measure
        vec![
            (1.0, 80),
            (0.5, 70),
            (0.5, 75),
            (1.0, 85),
            (0.5, 65),
            (0.5, 70),
        ]
    } else {
        // Standard pulse
        vec![(1.5, 80), (0.5, 60), (1.5, 78), (0.5, 55)]
    };

    for (dur, vel) in pattern {
        notes.push(TimedNote::new(root, dur, vel));
    }

    notes
}

/// Generate driving combat bass
fn generate_combat_bass(
    chord: &Chord,
    scale: &Scale,
    octave: u8,
    measure: usize,
) -> Vec<TimedNote> {
    let mut notes = Vec::new();
    let root = chord.root.with_octave(octave);
    let fifth = Note::new(scale.notes[4 % scale.notes.len()], octave);

    if measure % 2 == 0 {
        // Straight driving eighths
        for i in 0..8 {
            let note = if i % 2 == 0 { root } else { fifth };
            let vel = if i % 4 == 0 { 100 } else { 85 };
            notes.push(TimedNote::new(note, 0.5, vel));
        }
    } else {
        // Syncopated pattern
        notes.push(TimedNote::new(root, 0.5, 100));
        notes.push(TimedNote::new(root, 0.25, 90));
        notes.push(TimedNote::new(fifth, 0.25, 85));
        notes.push(TimedNote::new(root, 0.5, 95));
        notes.push(TimedNote::pause(0.25));
        notes.push(TimedNote::new(root, 0.25, 88));
        notes.push(TimedNote::new(fifth, 0.5, 92));
        notes.push(TimedNote::new(root, 0.5, 98));
        notes.push(TimedNote::new(fifth, 0.5, 90));
        notes.push(TimedNote::new(root, 0.5, 100));
    }

    notes
}

/// Generate arpeggiated harmony for atmosphere
fn generate_arpeggio(chord: &Chord, octave: u8, intensity: Intensity) -> Vec<TimedNote> {
    let mut notes = Vec::new();
    let chord_notes = chord.split_triad_to_notes();

    let (pattern, base_vel): (Vec<usize>, u8) = match intensity {
        Intensity::Calm => (vec![0, 1, 2, 1], 55), // Gentle up-down
        Intensity::Tension => (vec![0, 2, 1, 2, 0, 1], 65), // More movement
        Intensity::Combat => (vec![0, 1, 2, 2, 1, 0, 1, 2], 80), // Rapid
    };

    let note_duration = match intensity {
        Intensity::Calm => 1.0,
        Intensity::Tension => 0.66,
        Intensity::Combat => 0.5,
    };

    for (i, &idx) in pattern.iter().enumerate() {
        if idx < chord_notes.len() {
            let note = chord_notes[idx].with_octave(octave);
            let vel = base_vel + (i as u8 % 3) * 5;
            notes.push(TimedNote::new(note, note_duration, vel));
        }
    }

    notes
}

/// Generate pad/sustained harmony
fn generate_harmony_pad(
    chord: &Chord,
    octave: u8,
    intensity: Intensity,
    measure: usize,
) -> Vec<TimedNote> {
    let mut notes = Vec::new();

    match intensity {
        Intensity::Calm => {
            // Very sparse, only every other measure after intro
            if measure >= 2 && measure % 2 == 0 {
                let root = chord.root.with_octave(octave);
                notes.push(TimedNote::new(root, 4.0, 50));
            } else {
                notes.push(TimedNote::pause(4.0));
            }
        }
        Intensity::Tension => {
            // Building chords
            if measure >= 1 {
                let chord_notes = chord.split_triad_to_notes();
                // Layer the chord notes with slight stagger
                for (i, cn) in chord_notes.iter().take(2).enumerate() {
                    let note = cn.with_octave(octave);
                    let vel = 55 - (i as u8 * 5);
                    notes.push(TimedNote::new(note, 4.0, vel));
                }
            } else {
                notes.push(TimedNote::pause(4.0));
            }
        }
        Intensity::Combat => {
            // Staccato power chords
            let root = chord.root.with_octave(octave);
            let fifth_note_type = chord
                .notes
                .get(1)
                .map(|n| n.note_type)
                .unwrap_or(chord.root.note_type);
            let fifth = Note::new(fifth_note_type, octave);

            for i in 0..4 {
                let vel = if i % 2 == 0 { 80 } else { 70 };
                notes.push(TimedNote::new(root, 0.5, vel));
                notes.push(TimedNote::new(fifth, 0.5, vel - 5));
            }
        }
    }

    notes
}

/// Motifs for different intensities
fn get_motifs(intensity: Intensity) -> Vec<ScaleMotif> {
    match intensity {
        Intensity::Calm => vec![
            // Gentle, spacious motif
            ScaleMotif::new(vec![(0, 1.5, 70), (2, 1.0, 75), (4, 1.5, 72)]),
            // Descending lament
            ScaleMotif::new(vec![(4, 1.0, 75), (3, 1.0, 70), (2, 1.0, 68), (0, 1.0, 72)]),
        ],
        Intensity::Tension => vec![
            // Nervous, unsettled motif
            ScaleMotif::new(vec![
                (0, 0.5, 85),
                (1, 0.5, 90),
                (0, 0.5, 82),
                (2, 1.0, 95),
                (1, 0.5, 88),
                (0, 1.0, 80),
            ]),
            // Rising dread
            ScaleMotif::new(vec![
                (0, 0.75, 80),
                (1, 0.75, 85),
                (2, 0.75, 90),
                (3, 0.75, 95),
                (2, 1.0, 88),
            ]),
            // Tritone tension
            ScaleMotif::new(vec![
                (0, 0.5, 88),
                (3, 0.5, 92),
                (0, 0.5, 85),
                (3, 0.5, 90),
                (4, 1.0, 95),
                (0, 1.0, 82),
            ]),
        ],
        Intensity::Combat => vec![
            // Aggressive stab motif
            ScaleMotif::new(vec![
                (0, 0.25, 110),
                (0, 0.25, 100),
                (2, 0.25, 115),
                (0, 0.25, 105),
                (4, 0.5, 120),
                (2, 0.5, 110),
            ]),
            // Descending fury
            ScaleMotif::new(vec![
                (7, 0.25, 115),
                (6, 0.25, 110),
                (5, 0.25, 112),
                (4, 0.25, 108),
                (3, 0.25, 110),
                (2, 0.25, 105),
                (1, 0.25, 108),
                (0, 0.75, 120),
            ]),
            // Hammer pattern
            ScaleMotif::new(vec![
                (0, 0.25, 120),
                (0, 0.25, 110),
                (0, 0.25, 115),
                (2, 0.25, 118),
                (4, 0.5, 125),
                (0, 0.5, 115),
            ]),
        ],
    }
}

/// Generate melodic content for a measure
fn generate_melody(
    intensity: Intensity,
    scale: &Scale,
    chord: &Chord,
    octave: u8,
    measure: usize,
    total_measures: usize,
) -> Vec<TimedNote> {
    let mut notes = Vec::new();
    let motifs = get_motifs(intensity);
    let intro_measures = match intensity {
        Intensity::Calm => 4,
        Intensity::Tension => 2,
        Intensity::Combat => 0,
    };

    // Gradual introduction of melody
    if measure < intro_measures {
        notes.push(TimedNote::pause(4.0));
        return notes;
    }

    let melody_octave = octave + 1;
    let range = (melody_octave, melody_octave + 1);

    match intensity {
        Intensity::Calm => {
            // Sparse, contemplative melody
            if measure % 2 == 0 && !motifs.is_empty() {
                let motif = &motifs[measure % motifs.len()];
                let root = chord.root.with_octave(melody_octave);
                let generated = motif.generate(scale, root);
                for n in generated {
                    notes.push(n);
                }
            } else {
                // Sparse notes or rest
                if rand::random::<f32>() < 0.4 {
                    let note = scale.get_random_note(range);
                    notes.push(TimedNote::pause(1.0));
                    notes.push(TimedNote::new(note, 2.0, 65));
                    notes.push(TimedNote::pause(1.0));
                } else {
                    notes.push(TimedNote::pause(4.0));
                }
            }
        }
        Intensity::Tension => {
            // Building tension with more activity
            let motif = &motifs[measure % motifs.len()];
            let root = chord.root.with_octave(melody_octave);
            let generated = motif.generate(scale, root);

            // Add some randomization
            for (i, n) in generated.iter().enumerate() {
                if i < generated.len() - 1 && rand::random::<f32>() < 0.2 {
                    // Occasional grace note
                    notes.push(TimedNote::new(
                        scale.get_random_note(range),
                        0.125,
                        n.velocity.saturating_sub(20),
                    ));
                }
                notes.push(*n);
            }

            // Fill remaining time with passing tones or rests
            let total_dur: f32 = notes.iter().map(|n| n.duration).sum();
            if total_dur < 4.0 {
                let remaining = 4.0 - total_dur;
                if rand::random::<f32>() < 0.5 {
                    notes.push(TimedNote::pause(remaining));
                } else {
                    let note = scale.get_random_note(range);
                    notes.push(TimedNote::new(note, remaining, 70));
                }
            }
        }
        Intensity::Combat => {
            // Intense, relentless melody
            if measure % 2 == 0 {
                let motif = &motifs[measure % motifs.len()];
                let root = chord.root.with_octave(melody_octave);
                let generated = motif.generate(scale, root);
                for n in generated {
                    notes.push(n);
                }

                // Fill with rapid notes
                let dur: f32 = notes.iter().map(|n| n.duration).sum();
                let remaining = 4.0 - dur;
                let num_fills = (remaining / 0.5) as usize;
                for _ in 0..num_fills {
                    let note = scale.get_random_note(range);
                    notes.push(TimedNote::new(note, 0.5, 105 + (rand::random::<u8>() % 15)));
                }
            } else {
                // Chaotic fills with accents
                let patterns: Vec<(f32, u8)> = vec![
                    (0.25, 110),
                    (0.25, 95),
                    (0.5, 115),
                    (0.25, 100),
                    (0.25, 105),
                    (0.5, 118),
                    (0.25, 95),
                    (0.25, 108),
                    (0.5, 120),
                    (0.5, 100),
                    (0.5, 110),
                ];
                for (dur, vel) in patterns {
                    let note = scale.get_random_note(range);
                    notes.push(TimedNote::new(note, dur, vel));
                }
            }

            // Climax at the end
            if measure == total_measures - 1 {
                notes.clear();
                // Final dramatic phrase
                let final_motif = ScaleMotif::new(vec![
                    (0, 0.5, 127),
                    (4, 0.5, 125),
                    (7, 1.0, 127),
                    (4, 0.5, 120),
                    (0, 1.5, 127),
                ]);
                let root = chord.root.with_octave(melody_octave);
                notes = final_motif.generate(scale, root);
            }
        }
    }

    notes
}

/// Generate counter-melody or secondary melodic line
fn generate_counter_melody(
    intensity: Intensity,
    _scale: &Scale,
    chord: &Chord,
    octave: u8,
    measure: usize,
) -> Vec<TimedNote> {
    let mut notes = Vec::new();

    // Counter-melody only appears in later measures and more intense sections
    let start_measure = match intensity {
        Intensity::Calm => 6,
        Intensity::Tension => 4,
        Intensity::Combat => 2,
    };

    if measure < start_measure {
        notes.push(TimedNote::pause(4.0));
        return notes;
    }

    let counter_octave = octave;
    let chord_notes = chord.split_triad_to_notes();

    match intensity {
        Intensity::Calm => {
            // Simple held notes from chord
            if !chord_notes.is_empty() {
                let note = chord_notes[1 % chord_notes.len()].with_octave(counter_octave);
                notes.push(TimedNote::new(note, 4.0, 45));
            }
        }
        Intensity::Tension => {
            // Moving counter line
            if chord_notes.len() >= 2 {
                notes.push(TimedNote::new(
                    chord_notes[0].with_octave(counter_octave),
                    2.0,
                    55,
                ));
                notes.push(TimedNote::new(
                    chord_notes[1].with_octave(counter_octave),
                    2.0,
                    50,
                ));
            }
        }
        Intensity::Combat => {
            // Rhythmic counter hits
            for i in 0..4 {
                if !chord_notes.is_empty() {
                    let note = chord_notes[i % chord_notes.len()].with_octave(counter_octave);
                    notes.push(TimedNote::new(note, 0.5, 75));
                    notes.push(TimedNote::pause(0.5));
                }
            }
        }
    }

    notes
}

pub fn create_zombie_song(intensity: Intensity) -> Song {
    let octave = 3;
    let mut song = Song::new();
    let num_measures = 8;

    // Set tempo based on intensity
    let tempo = match intensity {
        Intensity::Calm => 65.0,
        Intensity::Tension => 90.0,
        Intensity::Combat => 135.0,
    };

    // Get the appropriate scale
    let scale = match intensity {
        Intensity::Calm => Scale::minor(Note::new(NoteType::A, octave)),
        Intensity::Tension => Scale::phrygian(Note::new(NoteType::A, octave)),
        Intensity::Combat => Scale::harmonic_minor(Note::new(NoteType::A, octave)),
    };

    // Get chord progression
    let chords = get_chord_progression(intensity, &scale, octave, num_measures);

    // Create lines
    let mut melody = Line::new();
    let mut counter_melody = Line::new();
    let mut harmony = Line::new();
    let mut arpeggio = Line::new();
    let mut bass = Line::new();

    let bass_octave = octave - 1;

    for measure in 0..num_measures {
        let chord = &chords[measure];

        // Bass line
        let bass_notes = match intensity {
            Intensity::Calm => generate_walking_bass(chord, &scale, bass_octave),
            Intensity::Tension => generate_ostinato_bass(chord, bass_octave, measure),
            Intensity::Combat => generate_combat_bass(chord, &scale, bass_octave, measure),
        };
        for n in bass_notes {
            bass.add_note(n);
        }

        // Harmony pad
        let harmony_notes = generate_harmony_pad(chord, octave, intensity, measure);
        for n in harmony_notes {
            harmony.add_note(n);
        }

        // Arpeggio layer (adds atmosphere)
        if measure >= 1 || intensity == Intensity::Combat {
            let arp_notes = generate_arpeggio(chord, octave, intensity);
            // Calculate duration before consuming the vector
            let dur: f32 = arp_notes.iter().map(|n| n.duration).sum();
            for n in arp_notes {
                arpeggio.add_note(n);
            }
            // Pad to fill measure if needed
            if dur < 4.0 {
                arpeggio.add_note(TimedNote::pause(4.0 - dur));
            }
        } else {
            arpeggio.add_note(TimedNote::pause(4.0));
        }

        // Melody
        let melody_notes = generate_melody(intensity, &scale, chord, octave, measure, num_measures);
        for n in melody_notes {
            melody.add_note(n);
        }

        // Counter melody
        let counter_notes = generate_counter_melody(intensity, &scale, chord, octave, measure);
        for n in counter_notes {
            counter_melody.add_note(n);
        }
    }

    song.tempo = tempo;
    song.add_line(melody);
    song.add_line(counter_melody);
    song.add_line(harmony);
    song.add_line(arpeggio);
    song.add_line(bass);
    song
}
