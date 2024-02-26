use std::fmt::{Display, Formatter};
use pancurses::{initscr, endwin, Input, noecho, Window, curs_set, Attribute};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use regex::Regex;
use serde::{Deserialize, Serialize};
use xz2::bufread::{XzEncoder, XzDecoder};

// idek
const SECTION_LABELS: [&str; 16] = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P"];

#[derive(Serialize, Deserialize)]
struct Song {
    title: String,
    sections: Vec<Section>,
}

impl Song {
    fn new() -> Self {
        Self {
            title: "untitled".to_string(),
            sections: vec![
                Section {
                    label: "A".to_string(),
                    bars: vec![
                        Bar::default()
                    ],
                    repeats: false,
                    wrap: 4,
                }
            ],
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Section {
    label: String,
    bars: Vec<Bar>,
    repeats: bool,
    wrap: usize, // bars
}

#[derive(Serialize, Deserialize)]
struct Bar {
    beats: usize,
    subdivision: usize,
    chords: BTreeMap<usize, Chord>, // position in subdivisions
}

impl Default for Bar {
    fn default() -> Self {
        Bar {
            beats: 4,
            subdivision: 4,
            chords: BTreeMap::new(),
        }
    }
}

impl Bar {
    fn new(beats: usize, subdivision: usize) -> Self {
        Bar {
            beats,
            subdivision,
            chords: BTreeMap::new(),
        }
    }
    fn get_chord(&self, subdivision: usize) -> Option<&Chord> {
        for (i, c) in &self.chords {
            if *i == subdivision {
                return Some(c);
            }
        }
        None
    }
    fn get_chord_mut(&mut self, subdivision: usize) -> Option<&mut Chord> {
        for (i, c) in &mut self.chords {
            if *i == subdivision {
                return Some(c);
            }
        }
        None
    }
    fn try_reduce_subdivision(&mut self) -> bool {
        if self.subdivision == 1 {
            return false;
        }
        let new = self.subdivision / 2;
        if self.chords.len() > new {
            return false; // won't fit
        }
        for chord_i in self.chords.clone().into_keys() {
            let chord = self.chords.remove(&chord_i).unwrap();
            let new_i = chord_i / 2;
            self.chords.insert(new_i, chord);
        }
        self.subdivision = new;
        true
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Chord {
    note: Note,
    accidental: Accidental,
    quality: Quality,
    over: Option<Note>,
    special: bool,
    question: bool,
}

impl Chord {
    fn parse(s: &str) -> Result<Self, ()> {
        // silly regex i partially stole from some random place (https://regex101.com/r/T5GuGD/1 is my copy)
        // groups:
        // 1. note
        // 2. accidental
        // 3. combined quality + extensions (we use)
        // 4. quality alone
        // 5. extensions alone
        // 6. over
        // 7. special (!)
        // 8. question (?)
        let re = Regex::new(r"([CDEFGABcdefgab])([#b])?((M|-|\+|\^|m|o|aug|dim|sus|add|hd)?(6|7|9|11|13|5|b5)?)(/[CDEFGABcdefgab])?(!)?(\?)?").unwrap();
        let caps = re.captures(s).ok_or(())?;

        let note_s = caps.get(1).ok_or(())?;
        let note = Note::try_from(note_s.as_str().chars().nth(0).unwrap()).unwrap();
        let accidental = if let Some(accidental_s) = caps.get(2) {
            match accidental_s.as_str() {
                "#" => Accidental::Sharp,
                "b" => Accidental::Flat,
                _ => unreachable!()
            }
        } else {
            Accidental::None
        };

        let quality = if let Some(quality_s) = caps.get(3) {
            match quality_s.as_str() {
                "" => Quality::Maj, // idk why but that's what it does
                "-" | "m" => Quality::Min,
                "7" => Quality::Dom7,
                "-7" | "m7" => Quality::Min7,
                "^" | "^7" | "M7" => Quality::Maj7,
                "dim" | "o" => Quality::Dim,
                "dim7" | "o7" => Quality::Dim7,
                "hd" => Quality::HalfDim,

                // TODO
                _ => return Err(())
            }
        } else {
            Quality::Maj
        };


        Ok(Chord {
            note,
            accidental,
            quality,
            over: None,
            special: caps.get(7).is_some(),
            question: caps.get(8).is_some(),
        })
    }
}

impl Display for Chord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        dbg!(&self);
        write!(f, "{}{}{}", self.note, self.accidental, self.quality)?;
        if let Some(n) = &self.over {
            write!(f, "/{}", n)?;
        }
        if self.special {
            write!(f, "!")?;
        }
        if self.question {
            write!(f, "?")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum Note {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

impl TryFrom<char> for Note {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value.to_ascii_uppercase() {
            'A' => Ok(Self::A),
            'B' => Ok(Self::B),
            'C' => Ok(Self::C),
            'D' => Ok(Self::D),
            'E' => Ok(Self::E),
            'F' => Ok(Self::F),
            'G' => Ok(Self::G),
            _ => Err(())
        }
    }
}


impl Display for Note {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Note::A => 'A',
            Note::B => 'B',
            Note::C => 'C',
            Note::D => 'D',
            Note::E => 'E',
            Note::F => 'F',
            Note::G => 'G',
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Accidental {
    None,
    Sharp,
    Flat,
}

impl Display for Accidental {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if matches!(self, Accidental::None) {
            return Ok(());
        }
        write!(f, "{}", match self {
            Accidental::None => unreachable!(),
            Accidental::Sharp => '#',
            Accidental::Flat => 'b'
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Quality {
    Maj,
    Min,
    Dom7,
    Maj7,
    Min7,
    Dim,
    Dim7,
    HalfDim,
    Aug,
    Dom9,
    Maj9,
    Min9,
    Flat9,
    Sharp9,
    Maj11,
    Sharp11,
    Dom13,
    Maj13,
    Flat13,
    Sus,
    Sus4,
    Sus2,
    // more complex chords out of scope :) (those r all i could think of that i use off the top of my head)
}

impl Display for Quality {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Quality::Maj => " ",
            Quality::Min => "-",
            Quality::Dom7 => "7",
            Quality::Maj7 => "^",
            Quality::Min7 => "-7",
            Quality::Dim => "o",
            Quality::Dim7 => "o7",
            Quality::HalfDim => "m7b7",
            Quality::Aug => "+",
            Quality::Dom9 => "9",
            Quality::Maj9 => "^9",
            Quality::Min9 => "-9",
            Quality::Flat9 => "b9",
            Quality::Sharp9 => "#9",
            Quality::Maj11 => "^11",
            Quality::Sharp11 => "#11",
            Quality::Dom13 => "13",
            Quality::Maj13 => "^13",
            Quality::Flat13 => "b13",
            Quality::Sus => "sus",
            Quality::Sus4 => "sus4",
            Quality::Sus2 => "sus2",
        })
    }
}

#[derive(Default, Debug, Copy, Clone)]
struct CursorPos {
    section: usize,
    bar: usize,
    subdivision: usize,
}

struct Toast {
    message: Option<String>,
    ticks: u32,
}
impl Default for Toast {
    fn default() -> Self {
        Toast {
            message: None,
            ticks: 0,
        }
    }
}

struct State {
    win: Window,
    song: Song,
    cursor: CursorPos,
    should_clear: bool,
    should_quit: bool,
    toast: Toast
}

impl State {
    fn schedule_clear(&mut self) {
        self.should_clear = true;
    }
    fn quit(&mut self) {
        self.should_quit = true;
    }
    fn find_cursor(&self) -> (i32, i32) {
        let mut ypos: i32 = 2;
        let mut xpos: i32 = 1;
        for s in self.song.sections.iter().take(self.cursor.section) {
            let x = ((s.bars.len() - 1) / s.wrap) as i32;
            ypos += x + 3;
        }
        ypos += 1;
        let wrap = self.song.sections[self.cursor.section].wrap;
        let col_widths = self.calc_widths(self.current_section());

        for i in 0..=self.cursor.bar {
            let width = col_widths[i % wrap] as i32;
            if i % wrap == 0 && i > 0 {
                ypos += 1;
                xpos = 1;
            }
            if i < self.cursor.bar {
                xpos += 1 + width * self.song.sections[self.cursor.section].bars[i].subdivision as i32;
            } else {
                xpos += width * self.cursor.subdivision as i32;
            }
        }

        (ypos, xpos)
    }
    fn calc_widths(&self, section: &Section) -> Vec<usize> {
        let wrap = section.wrap;
        let mut widths = vec![0; wrap];

        for (i, bar) in section.bars.iter().enumerate() {
            let idx = i % wrap;
            for subdivision in 0..bar.subdivision {
                if let Some(chord) = bar.get_chord(subdivision) {
                    let chord_str = format!("{} ", chord);
                    widths[idx] = chord_str.chars().count().max(widths[idx]);
                } else {
                    widths[idx] = 2.max(widths[idx]); // minimum width
                }
            }
        }
        widths
    }
    fn draw(&mut self) {
        if self.should_clear {
            self.win.clear();
        }

        // Header
        self.win.mvprintw(0, 0, "SONG: ");
        self.win.printw(&self.song.title);


        for (section_i, section) in self.song.sections.iter().enumerate() {
            let mut ypos = 1;

            for s in self.song.sections.iter().take(section_i) {
                let x = ((s.bars.len() - 1) / s.wrap) as i32;
                ypos += x + 3;
            }
            ypos += 1;
            self.win.mvaddch(ypos, 0, '[');
            self.win.addstr(&section.label);
            self.win.addch(']');
            ypos += 1;
            self.win.mv(ypos, 0);
            let col_widths = self.calc_widths(section);
            for (bar_i, bar) in section.bars.iter().enumerate() {
                if bar_i % section.wrap == 0 && bar_i > 0 {
                    self.win.addch('|'); // terminating
                    ypos += 1; // wow this code is gonna suck
                    self.win.mv(ypos, 0);
                }
                self.win.addch('|');
                for s in 0..bar.subdivision {
                    let selected =
                        if self.cursor.section == section_i && self.cursor.bar == bar_i && self.cursor.subdivision == s {
                            self.win.attron(Attribute::Reverse);
                            true
                        } else {
                            false
                        };

                    let col_width = col_widths[bar_i % section.wrap];

                    if let Some(chord) = bar.get_chord(s) {
                        // print chord
                        let chord_str = format!("{}", chord);
                        self.win.addstr(&chord_str);
                        // fill remaining space
                        self.win.addstr(" ".repeat(col_width - chord_str.len()));
                    } else if self.cursor.section == section_i && self.cursor.bar == bar_i {
                        self.win.addstr(".");
                        self.win.addstr(" ".repeat(col_width - 1));
                    } else {
                        self.win.addstr(" ".repeat(col_width));
                    }

                    if selected {
                        self.win.attroff(Attribute::Reverse);
                    }
                }
            }
            self.win.addch('|'); // terminating
            self.win.addstr(" ".repeat((self.win.get_max_x() - self.win.get_cur_x() - 1) as usize));
        }
        self.draw_toast();
        self.win.refresh();
    }
    fn current_section(&self) -> &Section {
        &self.song.sections[self.cursor.section]
    }
    fn current_section_mut(&mut self) -> &mut Section {
        &mut self.song.sections[self.cursor.section]
    }
    fn next_bar(&mut self) {
        if self.cursor.bar + 1 == self.current_section().bars.len() {
            self.cursor.subdivision = self.current_section().bars.last().unwrap().subdivision - 1;
            return;
        }
        self.cursor.bar += 1;
    }
    fn next_or_create_bar(&mut self) {
        let cursor = self.cursor;
        let section = self.current_section();
        if section.bars.is_empty() {
            self.current_section_mut().bars.push(Bar::default());
            self.cursor.bar = 0;
            self.cursor.subdivision = 0;
            return;
        }

        let previous = section.bars.last().unwrap();
        let new = Bar::new(previous.beats, previous.subdivision);
        if section.bars.len() == cursor.bar + 1 && self.song.sections.len() == cursor.section + 1 {
            // last bar last section
            self.current_section_mut().bars.push(new);
            self.cursor.bar += 1;
            self.cursor.subdivision = 0;
            self.win.clear();
        } else if section.bars.len() == cursor.bar + 1 {
            // last bar not last section
            self.next_or_create_section();
        } else {
            self.cursor.bar += 1;
            self.cursor.subdivision = 0;
        }
    }
    fn prev_bar(&mut self) {
        if self.cursor.bar == 0 && self.cursor.subdivision > 0 {
            self.cursor.subdivision = 0;
            return;
        } else if self.cursor.bar == 0 && self.cursor.subdivision == 0 {
            self.prev_section();
        }

        self.cursor.bar = self.cursor.bar.saturating_sub(1);
    }
    fn next_subdivision(&mut self) {
        let current_bar = self.cursor.bar;
        let current_sub = self.cursor.subdivision;
        let section = self.current_section();

        if current_sub + 1 == section.bars[current_bar].subdivision {
            self.next_or_create_bar();
        } else {
            self.cursor.subdivision += 1;
        }
    }
    fn prev_subdivision(&mut self) {
        let cursor = self.cursor;

        if cursor.subdivision == 0 {
            self.prev_bar();
            let current_bar = self.cursor.bar;
            self.cursor.subdivision = self.current_section().bars[current_bar].subdivision - 1;
        } else {
            self.cursor.subdivision -= 1;
        }
    }
    fn chord_input(&mut self, first: Option<char>) -> Result<String, ()> {
        let mut buf = String::with_capacity(8);
        if let Some(f) = first {
            buf.push(f);
        }
        let mut finished = false;
        // find current cursor position
        let (y, x) = self.find_cursor();

        self.win.attron(Attribute::Reverse);
        while !finished {
            self.win.mvaddstr(y, x, &buf);
            let ch = self.win.getch();
            if let Some(Input::Character(c)) = ch {
                if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() {
                    buf.push(c);
                } else if c.is_whitespace() {
                    if c == ' ' {
                        self.next_subdivision();
                    } else if c == '\t' {
                        self.next_or_create_bar();
                    }
                    finished = true;
                } else if c == '\u{8}' {
                    buf.pop();
                    self.win.mvaddstr(y, x, &buf);
                    self.win.addch(' ');
                } else {
                    finished = true;
                }
            } else {
                finished = true;
            }
        }
        self.win.attroff(Attribute::Reverse);
        Ok(buf)
    }
    fn input_or_edit_in_place_chord(&mut self, first: char) {
        let Ok(note) = Note::try_from(first) else { return; };

        let cursor = self.cursor;
        // if let Some(prev_chord) = self.current_section_mut().bars[cursor.bar].get_chord_mut(cursor.subdivision) {
        //     // already a chord there
        //     // just change the root
        //     prev_chord.note = note;
        //     return;
        // }

        let new = self.chord_input(Some(first)).unwrap();
        if let Ok(chord) = Chord::parse(&new) {
            self.current_section_mut().bars[cursor.bar].chords.insert(cursor.subdivision, chord);
        }
    }
    fn do_command_line(&mut self) {
        let mut buf = String::new();

        let mut finished = false;
        let y= self.win.get_max_y() - 1;
        let x = 1;
        self.win.attron(Attribute::Reverse);
        self.win.mvaddch(y, 0,':');

        while !finished {
            self.win.mvaddstr(y, x, &buf);
            self.win.hline(' ', self.win.get_max_x() - buf.len() as i32);
            let ch = self.win.getch();
            if let Some(Input::Character(c)) = ch {
                if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() {
                    buf.push(c);
                } else if c == '\u{8}' {
                    buf.pop();
                    self.win.mvaddstr(y, x, &buf);
                    self.win.addch(' ');
                } else if c == ' '{
                    // autoexpand stuff
                    if buf == "t" {
                        buf = "title ".to_string();
                    } else if buf == "q" {
                        buf = "quit".to_string();
                    }else {
                        buf.push(' ');
                    }
                }
                 else
                 {
                    finished = true;
                }
            } else {
                finished = true;
            }
        }
        self.win.attroff(Attribute::Reverse);
        // now parse
        if buf.is_empty() {
            return
        }
        let components = buf.split_ascii_whitespace().collect::<Vec<&str>>();
        if components.first() == Some(&"title") && components.get(1).is_some() {
            // set title
            let title = components.get(1..).unwrap().join(" ");
            self.song.title = title;
            self.schedule_clear();
            self.toast(&format!("Set title to '{}'.", self.song.title));
        } else if components.first() == Some(&"quit") || components.first() == Some(&"q") {
            self.quit();
        }
    }

    fn draw_toast(&mut self) {
        if let Some(message) = &self.toast.message {
            if self.toast.ticks == 0 {
                return;
            }
            self.win.attron(Attribute::Reverse);
            self.win.mvaddstr(self.win.get_max_y() - 1, 0, message);
            self.win.attroff(Attribute::Reverse);
            self.toast.ticks -= 1;
        }
    }

    fn toast(&mut self, message: &str) {
        self.toast.message = Some(message.to_owned());
        self.toast.ticks = 2;
    }

    fn delete_chord_or_empty_bar(&mut self) {
        let cursor = self.cursor;
        let section = self.current_section_mut();
        let current_bar = &section.bars[cursor.bar];
        if current_bar.chords.is_empty() && section.bars.len() > 1 {
            section.bars.remove(cursor.bar);
            // put the cursor somewhere nice
            if cursor.bar >= section.bars.len() {
                self.cursor.bar -= 1;
            }
            self.schedule_clear();

        } else {
            section.bars[cursor.bar].chords.remove(&cursor.subdivision);
        }
    }
    fn next_or_create_section(&mut self) {
        if self.cursor.section + 1 < self.song.sections.len() {
            // next
            self.cursor.section += 1;
            self.cursor.bar = 0;
            self.cursor.subdivision = 0;
            return;
        }
        // create
        let previous = self.song.sections.last().unwrap();
        let new = Section {
            label: SECTION_LABELS.iter()
                .position(|&x| x == previous.label)
                .map(|n| *SECTION_LABELS.get(n + 1).unwrap_or(&"?"))
                .unwrap_or("?").to_owned(),
            bars: vec![Bar::new(
                previous.bars.last().unwrap().beats,
                previous.bars.last().unwrap().subdivision,
            )],
            repeats: false,
            wrap: previous.wrap,
        };
        self.song.sections.push(new);
        self.cursor.section += 1;
        self.cursor.bar = 0;
        self.cursor.subdivision = 0;
    }
    fn prev_section(&mut self) {
        if self.cursor.section > 0 {
            self.cursor.section -= 1;
            self.cursor.bar = self.song.sections[self.cursor.section].bars.len();
        }
    }
    fn save_to_disk(&self) {
        let encoded: Vec<u8> = bincode::serialize(&self.song).unwrap();
        let mut compressor = XzEncoder::new(encoded.as_slice(), 9);
        let mut compressed: Vec<u8> = vec![];
        compressor.read_to_end(&mut compressed).unwrap();
        fs::write("./out.chartz", compressed).expect("Unable to write file");
    }
}

fn main() {
    let window = initscr();
    window.keypad(true);
    noecho();
    curs_set(0);

    let mut state = State {
        win: window,
        song: Song::new(),
        cursor: CursorPos::default(),
        should_clear: true,
        should_quit: false,
        toast: Toast::default(),
    };

    loop {
        // draw
        state.draw();
        // get input
        match state.win.getch() {
            Some(Input::Character(c)) => {
                if c == '\t' {
                    state.next_or_create_bar();
                    continue;
                }
                if c == ' ' {
                    state.next_subdivision();
                    continue;
                }
                if c == 's' {
                    state.next_or_create_section();
                }
                if c == ':' {
                    println!("meow");
                    state.do_command_line();
                }
                state.input_or_edit_in_place_chord(c);
            }
            Some(Input::KeyDC) => {
                // DEL
                state.delete_chord_or_empty_bar();
            }
            Some(Input::KeyF1) => {
                state.song.sections[state.cursor.section].bars[state.cursor.bar].try_reduce_subdivision();
                state.win.touch();
            }
            Some(Input::KeyF12) => {
                state.save_to_disk();
            }
            Some(Input::KeyF4) => {
                state.next_or_create_bar();
            }
            Some(Input::KeyF3) => {
                state.prev_bar();
            }
            Some(Input::KeyRight) => {
                state.next_subdivision();
            }
            Some(Input::KeyLeft) => {
                state.prev_subdivision();
            }
            Some(Input::KeyUp) => {
                for _ in 0..state.current_section_mut().wrap {
                    state.prev_bar();
                }
            }
            Some(Input::KeyDown) => {
                for _ in 0..state.current_section_mut().wrap {
                    state.next_bar();
                }
            }
            Some(input) => {}
            None => ()
        }
        if state.should_quit {
            break
        }
    }
    endwin();
}
