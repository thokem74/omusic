use crate::theme::Theme;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use omusic::{Album, Browse, Command, Effect, Repeat, SearchResult, Session};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

pub struct Ui {
    pub browse: Browse,
    queue: bool,
    queue_index: usize,
    search: Option<String>,
    result_index: usize,
    help: bool,
    errors: bool,
    folder: Option<String>,
}

impl Ui {
    pub fn new(browse: Browse) -> Self {
        Self {
            browse,
            queue: false,
            queue_index: 0,
            search: None,
            result_index: 0,
            help: false,
            errors: false,
            folder: None,
        }
    }
    fn albums<'a>(&self, session: &'a Session) -> Vec<&'a Album> {
        session
            .collection()
            .albums
            .iter()
            .filter(|a| {
                a.artist == self.browse.artist
                    || a.tracks.iter().any(|id| {
                        session
                            .collection()
                            .track(id)
                            .is_some_and(|t| t.contributors.contains(&self.browse.artist))
                    })
            })
            .collect()
    }
    pub fn sync(&mut self, session: &Session) {
        let artists = session.collection().artists();
        if !artists.contains(&self.browse.artist)
            && let Some(artist) = artists.first()
        {
            self.browse.artist = artist.clone();
        }
        let albums = self.albums(session);
        if !albums.iter().any(|a| a.id == self.browse.album)
            && let Some(album) = albums.first()
        {
            self.browse.album = album.id.clone();
            self.browse.track = 0;
        }
        if let Some(album) = session.collection().album(&self.browse.album) {
            self.browse.track = self.browse.track.min(album.tracks.len().saturating_sub(1));
        }
        self.browse.pane = self.browse.pane.min(2);
        self.queue_index = self
            .queue_index
            .min(session.state().queue.len().saturating_sub(1));
    }
    pub fn key(&mut self, session: &mut Session, key: KeyEvent) -> Result<Vec<Effect>> {
        let mut command = None;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return session.dispatch(Command::Quit);
        }
        if let Some(text) = &mut self.folder {
            match key.code {
                KeyCode::Esc => self.folder = None,
                KeyCode::Backspace => {
                    text.pop();
                }
                KeyCode::Char(c) => text.push(c),
                KeyCode::Enter => {
                    let text = text.clone();
                    self.folder = None;
                    let path = if let Some(rest) = text.strip_prefix("~/") {
                        std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                            .join(rest)
                    } else {
                        text.into()
                    };
                    let path = if path.is_absolute() {
                        path
                    } else {
                        std::env::current_dir()?.join(path)
                    };
                    let mut roots = session.state().roots.clone();
                    if !roots.contains(&path) {
                        roots.push(path);
                    }
                    session.rescan(roots)?;
                }
                _ => {}
            }
            return Ok(vec![]);
        }
        if let Some(query) = &mut self.search {
            match key.code {
                KeyCode::Esc => self.search = None,
                KeyCode::Backspace => {
                    query.pop();
                    self.result_index = 0;
                }
                KeyCode::Char(c) => {
                    query.push(c);
                    self.result_index = 0;
                }
                KeyCode::Down => {
                    self.result_index =
                        (self.result_index + 1).min(session.search(query).len().saturating_sub(1))
                }
                KeyCode::Up => self.result_index = self.result_index.saturating_sub(1),
                KeyCode::Enter => {
                    if let Some(result) = session.search(query).get(self.result_index) {
                        match result {
                            SearchResult::Artist(artist) => {
                                self.browse.artist = artist.clone();
                                self.browse.album.clear();
                                self.browse.pane = 1;
                            }
                            SearchResult::Album(id) => {
                                if let Some(album) = session.collection().album(id) {
                                    self.browse.artist = album.artist.clone();
                                }
                                self.browse.album = id.clone();
                                self.browse.pane = 2;
                            }
                            SearchResult::Track(id) => {
                                if let Some(t) = session.collection().track(id) {
                                    self.browse.album = t.album_id.clone();
                                    if let Some(a) = session.collection().album(&t.album_id) {
                                        self.browse.artist = a.artist.clone();
                                        self.browse.track =
                                            a.tracks.iter().position(|id| id == &t.id).unwrap_or(0);
                                    }
                                }
                                command = Some(Command::PlayTrack(id.clone()));
                            }
                        }
                        self.search = None;
                    }
                }
                _ => {}
            }
        } else if self.help || self.errors {
            self.help = false;
            self.errors = false;
        } else {
            match key.code {
                KeyCode::Char('q') => command = Some(Command::Quit),
                KeyCode::Char('?') => self.help = true,
                KeyCode::Char('e') => self.errors = true,
                KeyCode::Char('f') => self.folder = Some(String::new()),
                KeyCode::Char('/') => {
                    self.search = Some(String::new());
                    self.result_index = 0;
                }
                KeyCode::Char('Q') => self.queue = !self.queue,
                KeyCode::Esc => self.queue = false,
                KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                    self.browse.pane = (self.browse.pane + 1) % 3
                }
                KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
                    self.browse.pane = (self.browse.pane + 2) % 3
                }
                KeyCode::Down | KeyCode::Char('j') => self.move_selection(session, 1),
                KeyCode::Up | KeyCode::Char('k') => self.move_selection(session, -1),
                KeyCode::Char(' ') => command = Some(Command::TogglePlay),
                KeyCode::Char('n') => command = Some(Command::Next),
                KeyCode::Char('N') => command = Some(Command::Previous),
                KeyCode::Char(']') => command = Some(Command::Seek(session.state().position + 5.0)),
                KeyCode::Char('[') => command = Some(Command::Seek(session.state().position - 5.0)),
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    command = Some(Command::Volume(session.state().volume + 5.0))
                }
                KeyCode::Char('-') => command = Some(Command::Volume(session.state().volume - 5.0)),
                KeyCode::Char('z') => command = Some(Command::Shuffle(!session.state().shuffle)),
                KeyCode::Char('r') => {
                    command = Some(Command::Repeat(match session.state().repeat {
                        Repeat::Off => Repeat::Track,
                        Repeat::Track => Repeat::Queue,
                        Repeat::Queue => Repeat::Off,
                    }))
                }
                KeyCode::Char('R') => session.rescan(session.state().roots.clone())?,
                KeyCode::Char('C') if self.queue => command = Some(Command::Clear),
                KeyCode::Char('d') if self.queue => {
                    command = Some(Command::Remove(self.queue_index))
                }
                KeyCode::Char('J') if self.queue => {
                    let to =
                        (self.queue_index + 1).min(session.state().queue.len().saturating_sub(1));
                    command = Some(Command::Move {
                        from: self.queue_index,
                        to,
                    });
                    self.queue_index = to;
                }
                KeyCode::Char('K') if self.queue => {
                    let to = self.queue_index.saturating_sub(1);
                    command = Some(Command::Move {
                        from: self.queue_index,
                        to,
                    });
                    self.queue_index = to;
                }
                KeyCode::Char('a') => {
                    command = if self.browse.pane == 2 {
                        session
                            .collection()
                            .album(&self.browse.album)
                            .and_then(|a| a.tracks.get(self.browse.track))
                            .cloned()
                            .map(Command::AppendTrack)
                    } else {
                        Some(Command::AppendAlbum(self.browse.album.clone()))
                    };
                }
                KeyCode::Char('P') => {
                    command = Some(Command::PlayAlbum {
                        id: self.browse.album.clone(),
                        start: 0,
                    })
                }
                KeyCode::Enter if self.queue && !session.state().queue.is_empty() => {
                    command = Some(Command::PlayQueue(self.queue_index))
                }
                KeyCode::Enter if self.browse.pane < 2 => self.browse.pane += 1,
                KeyCode::Enter => {
                    command = Some(Command::PlayAlbum {
                        id: self.browse.album.clone(),
                        start: self.browse.track,
                    })
                }
                _ => {}
            }
        }
        self.sync(session);
        let effects = if let Some(command) = command {
            session.dispatch(command)?
        } else {
            vec![]
        };
        session.dispatch(Command::Browse(self.browse.clone()))?;
        Ok(effects)
    }
    fn move_selection(&mut self, session: &Session, delta: isize) {
        let move_index = |current: usize, len: usize| {
            current
                .saturating_add_signed(delta)
                .min(len.saturating_sub(1))
        };
        if self.queue {
            self.queue_index = move_index(self.queue_index, session.state().queue.len());
            return;
        }
        match self.browse.pane {
            0 => {
                let artists = session.collection().artists();
                let current = artists
                    .iter()
                    .position(|a| a == &self.browse.artist)
                    .unwrap_or(0);
                if let Some(artist) = artists.get(move_index(current, artists.len())) {
                    self.browse.artist = artist.clone();
                    self.browse.album.clear();
                    self.browse.track = 0;
                }
            }
            1 => {
                let albums = self.albums(session);
                let current = albums
                    .iter()
                    .position(|a| a.id == self.browse.album)
                    .unwrap_or(0);
                if let Some(album) = albums.get(move_index(current, albums.len())) {
                    self.browse.album = album.id.clone();
                    self.browse.track = 0;
                }
            }
            _ => {
                let len = session
                    .collection()
                    .album(&self.browse.album)
                    .map_or(0, |a| a.tracks.len());
                self.browse.track = move_index(self.browse.track, len);
            }
        }
        self.sync(session);
    }
    pub fn draw(&self, frame: &mut Frame, session: &Session, theme: Theme) {
        let area = frame.area();
        let style = Style::default().bg(theme.background).fg(theme.foreground);
        frame.render_widget(Block::default().style(style), area);
        let rows = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
        let state = session.state();
        frame.render_widget(
            Paragraph::new(format!(
                " omusic.   Library                     {} artists · {} albums · {} tracks{}",
                session.collection().artists().len(),
                session.collection().albums.len(),
                session.collection().tracks.len(),
                if session.is_scanning() {
                    " · scanning…"
                } else {
                    ""
                }
            ))
            .style(Style::default().fg(theme.accent)),
            rows[0],
        );
        if self.queue {
            self.draw_queue(frame, session, theme, rows[1]);
        } else if session.collection().albums.is_empty() {
            frame.render_widget(Paragraph::new(if session.is_scanning(){"\n  Scanning your collection…\n  Cached music will appear here."}else{"\n  No MP3 tracks found.\n\n  Press f to add a music folder, R to rescan, or ? for help."}).block(Block::bordered().title(" Collection ")),rows[1]);
        } else {
            let columns = if area.width < 80 {
                vec![rows[1]]
            } else {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(23),
                        Constraint::Percentage(30),
                        Constraint::Percentage(47),
                    ])
                    .split(rows[1])
                    .to_vec()
            };
            for (index, column) in columns.iter().enumerate() {
                let pane = if columns.len() == 1 {
                    self.browse.pane
                } else {
                    index
                };
                self.draw_pane(frame, session, theme, *column, pane);
            }
        }
        let track = state.current_track();
        let status = if state.playing { "▶" } else { "Ⅱ" };
        let title = track.map_or("Nothing queued".into(), |t| {
            format!("{} — {} / {}", t.title, t.artist, t.album)
        });
        let progress = if state.duration > 0.0 {
            ((state.position / state.duration) * 20.0).clamp(0.0, 20.0) as usize
        } else {
            0
        };
        let bar = format!("{}{}", "━".repeat(progress), "─".repeat(20 - progress));
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!(" {status} {title}")),
                Line::from(format!(
                    " {} {bar} {}   vol {:.0}%   shuffle {}   repeat {:?}",
                    time(state.position),
                    time(state.duration),
                    state.volume,
                    if state.shuffle { "on" } else { "off" },
                    state.repeat
                )),
            ])
            .style(Style::default().fg(theme.green))
            .block(Block::default().borders(Borders::TOP)),
            rows[2],
        );
        frame.render_widget(
            Paragraph::new(if self.queue {
                " Q back  j/k move  Enter play  d remove  J/K reorder  C clear  q quit"
            } else {
                " ? help  / search  Tab/h/l pane  j/k move  Enter open/play  a add  Q queue  q quit"
            })
            .style(Style::default().fg(theme.accent)),
            rows[3],
        );
        frame.render_widget(
            Paragraph::new(format!(
                " {}",
                state
                    .errors
                    .last()
                    .map(String::as_str)
                    .unwrap_or("Read-only collection · R rescan · f add folder · e errors")
            ))
            .style(Style::default().fg(theme.muted)),
            rows[4],
        );
        if let Some(query) = &self.search {
            self.draw_search(frame, session, theme, query);
        }
        if self.help {
            popup(
                frame,
                " Help · any key to close ",
                "Tab / h l / ← →    switch pane\nj k / ↑ ↓         move selection\nEnter              open artist/album; play track\nP                  replace queue with selected album\na                  append selected album or track\nSpace              play / pause\nn / N              next / previous (restart after 3s)\n[ / ]              seek backward / forward 5s\n+ / -              volume\nz / r              shuffle / cycle repeat\nQ                  queue; d remove; J/K reorder; C clear\n/                  search collection (Esc closes)\nf / R              add folder / rescan\ne                  inspect errors\nq / Ctrl-C         quit and stop playback",
                theme,
            );
        }
        if self.errors {
            popup(
                frame,
                " Errors · any key to close ",
                &if state.errors.is_empty() {
                    "No errors recorded.".into()
                } else {
                    state.errors.join("\n")
                },
                theme,
            );
        }
        if let Some(folder) = &self.folder {
            popup(
                frame,
                " Add collection folder · Enter accept / Esc cancel ",
                &format!(
                    "Folder: {folder}\n\nAccepts an absolute path or ~/Music.\nExisting collection folders are retained."
                ),
                theme,
            );
        }
    }
    fn draw_pane(
        &self,
        frame: &mut Frame,
        session: &Session,
        theme: Theme,
        area: Rect,
        pane: usize,
    ) {
        let (title, items, selected) = match pane {
            0 => {
                let artists = session.collection().artists();
                let selected = artists.iter().position(|a| a == &self.browse.artist);
                (
                    " Artists ".to_string(),
                    artists.into_iter().map(ListItem::new).collect::<Vec<_>>(),
                    selected,
                )
            }
            1 => {
                let albums = self.albums(session);
                let selected = albums.iter().position(|a| a.id == self.browse.album);
                (
                    format!(" Albums · {} ", self.browse.artist),
                    albums
                        .into_iter()
                        .map(|a| {
                            ListItem::new(vec![
                                Line::from(a.title.clone()),
                                Line::from(Span::styled(
                                    if a.artist != self.browse.artist {
                                        format!("  Appears on · {}", a.artist)
                                    } else {
                                        format!(
                                            "  {} tracks{}",
                                            a.tracks.len(),
                                            if a.warning {
                                                " · ⚠ conflicting credits"
                                            } else {
                                                ""
                                            }
                                        )
                                    },
                                    Style::default().fg(theme.muted),
                                )),
                            ])
                        })
                        .collect(),
                    selected,
                )
            }
            _ => {
                let album = session.collection().album(&self.browse.album);
                let items = album
                    .into_iter()
                    .flat_map(|a| {
                        a.tracks
                            .iter()
                            .filter_map(move |id| session.collection().track(id).map(|t| (a, t)))
                    })
                    .enumerate()
                    .map(|(i, (a, t))| {
                        let contributes = a.artist != self.browse.artist
                            && t.contributors.contains(&self.browse.artist);
                        let playing = state_current_id(session).is_some_and(|id| id == t.id);
                        let text = format!(
                            "{:02} {}{}{}",
                            i + 1,
                            if contributes { "◆ " } else { "" },
                            t.title,
                            if t.artist != a.artist {
                                format!(" — {}", t.artist)
                            } else {
                                String::new()
                            }
                        );
                        ListItem::new(text).style(if playing {
                            Style::default().fg(theme.green)
                        } else {
                            Style::default()
                        })
                    })
                    .collect();
                (
                    format!(" Tracks · {} ", album.map_or("", |a| a.title.as_str())),
                    items,
                    Some(self.browse.track),
                )
            }
        };
        let block = Block::bordered()
            .title(title)
            .border_style(Style::default().fg(if pane == self.browse.pane {
                theme.accent
            } else {
                theme.muted
            }));
        frame.render_stateful_widget(
            List::new(items)
                .block(block)
                .highlight_style(
                    Style::default()
                        .bg(theme.selection)
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› "),
            area,
            &mut ListState::default().with_selected(selected),
        );
    }
    fn draw_queue(&self, frame: &mut Frame, session: &Session, theme: Theme, area: Rect) {
        let items = session.state().queue.iter().enumerate().map(|(i, t)| {
            ListItem::new(format!(
                "{} {:02}  {} — {}",
                if Some(i) == session.state().current {
                    "▶"
                } else {
                    " "
                },
                i + 1,
                t.title,
                t.artist
            ))
        });
        frame.render_stateful_widget(
            List::new(items)
                .block(Block::bordered().title(" Queue · d remove · J/K reorder · C clear "))
                .highlight_style(Style::default().bg(theme.selection).fg(theme.accent)),
            area,
            &mut ListState::default().with_selected(Some(self.queue_index)),
        );
    }
    fn draw_search(&self, frame: &mut Frame, session: &Session, theme: Theme, query: &str) {
        let area = center(frame.area(), 85, 85);
        frame.render_widget(Clear, area);
        let results = session.search(query);
        let items = results.iter().map(|result| {
            ListItem::new(match result {
                SearchResult::Artist(artist) => format!("ARTIST  {artist}"),
                SearchResult::Album(id) => {
                    session.collection().album(id).map_or(String::new(), |a| {
                        format!("ALBUM   {} — {}", a.title, a.artist)
                    })
                }
                SearchResult::Track(id) => {
                    session.collection().track(id).map_or(String::new(), |t| {
                        format!("TRACK   {} — {} / {}", t.title, t.artist, t.album)
                    })
                }
            })
        });
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::bordered()
                        .style(Style::default().bg(theme.background))
                        .title(format!(
                            " / {query} · {} results · Esc close ",
                            results.len()
                        )),
                )
                .highlight_style(Style::default().bg(theme.selection).fg(theme.accent)),
            area,
            &mut ListState::default().with_selected(Some(self.result_index)),
        );
    }
}

fn state_current_id(session: &Session) -> Option<&str> {
    session.state().current_track().map(|t| t.id.as_str())
}
fn time(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
fn center(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - height) / 2),
        Constraint::Percentage(height),
        Constraint::Min(0),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - width) / 2),
        Constraint::Percentage(width),
        Constraint::Min(0),
    ])
    .split(vertical[1])[1]
}
fn popup(frame: &mut Frame, title: &str, text: &str, theme: Theme) {
    let area = center(frame.area(), 85, 85);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(text.to_string())
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(theme.foreground).bg(theme.background))
            .block(Block::bordered().title(title)),
        area,
    );
}
