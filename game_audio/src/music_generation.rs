#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoteType {
    C = 1,
    Cs = 2,
    D = 3,
    Ds = 4,
    E = 5,
    F = 6,
    Fs = 7,
    G = 8,
    Gs = 9,
    A = 10,
    As = 11,
    B = 12,
}

impl From<u8> for NoteType {
    fn from(value: u8) -> Self {
        match value {
            1 => NoteType::C,
            2 => NoteType::Cs,
            3 => NoteType::D,
            4 => NoteType::Ds,
            5 => NoteType::E,
            6 => NoteType::F,
            7 => NoteType::Fs,
            8 => NoteType::G,
            9 => NoteType::Gs,
            10 => NoteType::A,
            11 => NoteType::As,
            12 => NoteType::B,
            _ => panic!("Invalid NoteType value"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Note {
    pub note_type: NoteType,
    pub octave: u8,
}

impl Note {
    pub fn new(note_type: NoteType, octave: u8) -> Self {
        Note { note_type, octave }
    }

    pub fn with_octave(&self, octave: u8) -> Self {
        Note {
            note_type: self.note_type,
            octave,
        }
    }
}

impl From<NoteType> for Note {
    fn from(note_type: NoteType) -> Self {
        Note {
            note_type,
            octave: 4,
        }
    }
}

impl From<Note> for NoteType {
    fn from(note: Note) -> Self {
        note.note_type
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScaleType {
    Major,
    Minor,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scale {
    pub root: Note,
    pub notes: Vec<NoteType>,
}

impl Scale {
    pub fn new(root: Note, notes: Vec<NoteType>) -> Self {
        Scale { root, notes }
    }

    /// Get a random note within the given range in the scale.
    /// octaves: (min_note_type, max_note_type)
    /// e.g. (4, 7)
    pub fn get_random_note(&self, octaves: (u8, u8)) -> Note {
        let scale_size = self.notes.len() as u8;
        let random_note = rand::random::<u8>() % scale_size;
        let random_octave = rand::random::<u8>() % (octaves.1 - octaves.0 + 1) + octaves.0;
        Note::new(self.notes[random_note as usize], random_octave)
    }

    // returns the note and the suggested length

    pub fn from_type(root: Note, scale_type: ScaleType) -> Self {
        match scale_type {
            ScaleType::Major => Self::major(root),
            ScaleType::Minor => Self::minor(root),
        }
    }

    pub fn shift_notes(notes: Vec<NoteType>, root: Note) -> Vec<NoteType> {
        let multiplier = root.note_type as u8 - 1;
        notes
            .iter()
            .map(|&note| NoteType::from(((note as u8 - 1 + multiplier) % 12) + 1))
            .collect()
    }

    pub fn major(root: Note) -> Self {
        let notes = vec![
            NoteType::C,
            NoteType::D,
            NoteType::E,
            NoteType::F,
            NoteType::G,
            NoteType::A,
            NoteType::B,
        ];
        Self::new(root, Self::shift_notes(notes, root))
    }

    pub fn minor(root: Note) -> Self {
        let notes = vec![
            NoteType::C,
            NoteType::D,
            NoteType::Ds,
            NoteType::F,
            NoteType::G,
            NoteType::Gs,
            NoteType::As,
        ];
        Self::new(root, Self::shift_notes(notes, root))
    }

    pub fn phrygian(root: Note) -> Self {
        let notes = vec![
            NoteType::C,
            NoteType::Cs,
            NoteType::Ds,
            NoteType::F,
            NoteType::G,
            NoteType::Gs,
            NoteType::As,
        ];
        Self::new(root, Self::shift_notes(notes, root))
    }

    pub fn harmonic_minor(root: Note) -> Self {
        let notes = vec![
            NoteType::C,
            NoteType::D,
            NoteType::Ds,
            NoteType::F,
            NoteType::G,
            NoteType::Gs,
            NoteType::B,
        ];
        Self::new(root, Self::shift_notes(notes, root))
    }
}

impl Note {
    pub fn to_midi(&self) -> i32 {
        (self.octave as i32 + 1) * 12 + self.note_type as i32
    }
}

/// A timed note is a note with a duration.
#[derive(Debug, Clone, Copy)]
pub struct TimedNote {
    pub note: Option<Note>,
    /// The duration in beats.
    pub duration: f32,
    /// The velocity (volume) of the note (0-127).
    pub velocity: u8,
}

impl From<Note> for TimedNote {
    fn from(note: Note) -> Self {
        TimedNote {
            note: Some(note),
            duration: 1.0,
            velocity: 100,
        }
    }
}

impl From<TimedNote> for Note {
    fn from(timed_note: TimedNote) -> Self {
        timed_note.note.unwrap()
    }
}

impl TimedNote {
    pub fn new(note: Note, duration: f32, velocity: u8) -> Self {
        TimedNote {
            note: Some(note),
            duration,
            velocity,
        }
    }

    pub fn pause(duration: f32) -> Self {
        TimedNote {
            note: None,
            duration,
            velocity: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chord {
    pub root: Note,
    pub notes: Vec<Note>,
}

impl Chord {
    pub fn new(root: Note, notes: Vec<Note>) -> Self {
        Chord { root, notes }
    }

    pub fn from_scale(scale: &Scale) -> Chord {
        let root_note = scale.root;
        let root_type = scale.notes[0];
        let third_type = scale.notes[(0 + 2) % scale.notes.len()];
        let fifth_type = scale.notes[(0 + 4) % scale.notes.len()];

        let root = Note::new(root_type, root_note.octave);

        let third_octave = if third_type < root_type {
            root_note.octave + 1
        } else {
            root_note.octave
        };
        let fifth_octave = if fifth_type < root_type {
            root_note.octave + 1
        } else {
            root_note.octave
        };

        Chord::new(
            root,
            vec![
                Note::new(third_type, third_octave),
                Note::new(fifth_type, fifth_octave),
            ],
        )
    }

    pub fn split_triad_to_notes(&self) -> Vec<Note> {
        let mut notes = Vec::new();
        notes.push(self.root);
        notes.extend(self.notes.iter().cloned());
        notes
    }

    // New chord progression methods moved from Scale
}

#[derive(Debug)]
pub struct Line {
    pub notes: Vec<TimedNote>,
}

impl Line {
    pub fn new() -> Self {
        Line { notes: Vec::new() }
    }
    pub fn add_note(&mut self, note: TimedNote) {
        self.notes.push(note);
    }
}

#[derive(Debug, Clone)]
pub struct ScaleMotif {
    pub notes: Vec<(i32, f32, u8)>, // (degree_offset, duration, velocity)
}

impl ScaleMotif {
    pub fn new(notes: Vec<(i32, f32, u8)>) -> Self {
        ScaleMotif { notes }
    }

    pub fn generate(&self, scale: &Scale, root: Note) -> Vec<TimedNote> {
        let root_idx = scale.notes.iter().position(|&t| t == root.note_type);

        if let Some(idx) = root_idx {
            self.notes
                .iter()
                .map(|(offset, duration, velocity)| {
                    let target_idx_isize = (idx as isize) + (*offset as isize);
                    let scale_len = scale.notes.len() as isize;

                    let octave_shift = target_idx_isize.div_euclid(scale_len);
                    let wrapped_idx = target_idx_isize.rem_euclid(scale_len);

                    let note_type = scale.notes[wrapped_idx as usize];
                    // safe cast assuming octave doesn't overflow
                    let new_octave = (root.octave as isize + octave_shift) as u8;

                    TimedNote::new(Note::new(note_type, new_octave), *duration, *velocity)
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

pub struct Song {
    pub lines: Vec<Line>,
    pub tempo: f32,
}

impl Song {
    pub fn new() -> Self {
        Song {
            lines: Vec::new(),
            tempo: 120.0,
        }
    }

    pub fn add_line(&mut self, line: Line) {
        self.lines.push(line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shift_notes() {
        let root = Note::new(NoteType::C, 4);
        let notes = vec![NoteType::C, NoteType::E, NoteType::G];
        let shifted = Scale::shift_notes(notes.clone(), root);
        // C major shifted by C (0) should be C major
        assert_eq!(shifted, vec![NoteType::C, NoteType::E, NoteType::G]);

        let root_d = Note::new(NoteType::D, 4);
        let shifted_d = Scale::shift_notes(notes.clone(), root_d);
        // C (1) -> D (3) (+2)
        // E (5) -> Fs (7) (+2)
        // G (8) -> A (10) (+2)
        assert_eq!(shifted_d, vec![NoteType::D, NoteType::Fs, NoteType::A]);

        // Test wrapping
        let root_b = Note::new(NoteType::B, 4);
        // B is 12. Multiplier is 11.
        // C(1) + 11 -> 12 (B)
        // E(5) + 11 -> 16 -> 4 (Ds)
        // G(8) + 11 -> 19 -> 7 (Fs)
        let shifted_b = Scale::shift_notes(notes.clone(), root_b);
        assert_eq!(shifted_b, vec![NoteType::B, NoteType::Ds, NoteType::Fs]);
    }
}
