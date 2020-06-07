use std::path::PathBuf;

use iced::{
    button, executor, scrollable, text_input, Align, Application, Button, Color, Column, Command,
    Container, Element, Length, Scrollable, Settings, Subscription, Text, TextInput,
};
use iced_native::window::Event as WindowEvent;
use iced_native::Event;
use walkdir::WalkDir;

mod api;
mod message;
mod scenes;
mod styles;
mod widgets;

use message::Message;
use scenes::Scenes;
use widgets::file::{self, File, FileMessage};

fn is_video(path: &PathBuf) -> bool {
    let guess = mime_guess::from_path(path);

    match guess.first() {
        Some(guess) => guess.to_string().starts_with("video/"),
        None => false,
    }
}

pub fn main() {
    let mut settings = Settings::default();

    settings.default_font = Some(include_bytes!("../fonts/SourceCodePro-Regular.ttf"));

    App::run(settings)
}

#[derive(Debug, Default)]
struct App {
    id_counter: u64,
    hovering_with_files: bool,
    files: Vec<File>,
    file_scrollable: scrollable::State,
    current_scene: Scenes,
    next_button: button::State,

    // Scenes::Index
    api_key_input: text_input::State,
    api_key: String,
}

impl App {
    fn get_id(&mut self) -> u64 {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }

    pub fn contains_path(&self, path: &PathBuf) -> bool {
        self.files.iter().find(|file| &file.path == path).is_some()
    }

    pub fn add_path(&mut self, path: PathBuf) -> Vec<Command<Message>> {
        let mut commands = Vec::new();

        if path.is_dir() {
            for entry in WalkDir::new(path) {
                let file_path = entry.unwrap().path().to_owned();

                if is_video(&file_path) && !self.contains_path(&file_path) {
                    let id = self.get_id();

                    commands.push(Command::perform(
                        File::analyze_file(id, file_path.clone()),
                        move |result| Message::FileAnalyzed(id, result),
                    ));

                    self.files.push(File {
                        id,
                        path: file_path,
                        ..Default::default()
                    });
                }
            }
        } else if is_video(&path) && !self.contains_path(&path) {
            let id = self.get_id();

            commands.push(Command::perform(
                File::analyze_file(id.clone(), path.clone()),
                move |result| Message::FileAnalyzed(id, result),
            ));

            self.files.push(File {
                id,
                path,
                ..Default::default()
            });
        }

        commands
    }
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (App, Command<Message>) {
        (App::default(), Command::none())
    }

    fn title(&self) -> String {
        String::from("mx")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::EventOccurred(event) => match event {
                Event::Window(WindowEvent::FileHovered(_)) => {
                    self.hovering_with_files = true;
                }
                Event::Window(WindowEvent::FilesHoveredLeft) => {
                    self.hovering_with_files = false;
                }
                Event::Window(WindowEvent::FileDropped(path)) => {
                    self.hovering_with_files = false;
                    let commands = self.add_path(path);

                    return Command::batch(commands);
                }
                _ => {}
            },
            Message::FileAnalyzed(id, result) => match result {
                Ok(analysis) => {
                    if let Some(file) = self.files.iter_mut().find(|file| file.id == id) {
                        file.update(FileMessage::Analyzed(analysis));
                    }
                }
                Err(_) => {}
            },
            Message::NextScene => {
                self.current_scene = Scenes::FileIndex;
            }
            Message::ApiKeyInputChanged(new_value) => {
                self.api_key = new_value;
            }
            Message::Noop => {}
            Message::FileMessage(id, message) => {
                if let Some(file) = self.files.iter_mut().find(|file| file.id == id) {
                    file.update(message);
                }
            }
        };

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_native::subscription::events().map(Message::EventOccurred)
    }

    fn view(&mut self) -> Element<Message> {
        match self.current_scene {
            Scenes::Index => {
                let mut button = Column::new()
                    .push(Text::new("spin-archive.org - MX").color(Color::WHITE))
                    .push(
                        TextInput::new(
                            &mut self.api_key_input,
                            "API key",
                            &self.api_key,
                            Message::ApiKeyInputChanged,
                        )
                        .style(styles::TextInput::Primary)
                        .padding(8),
                    )
                    .max_width(300)
                    .spacing(12)
                    .align_items(Align::Center);

                if self.api_key.len() > 5 {
                    button = button.push(
                        Button::new(&mut self.next_button, Text::new("Next"))
                            .padding(8)
                            .style(styles::Button::Primary)
                            .on_press(Message::NextScene),
                    );
                }

                let container = Container::new(button)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y();

                Container::new(container)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(styles::Container { hovered: false })
                    .into()
            }
            Scenes::FileIndex => {
                let is_empty = self.files.is_empty();

                let file_index = file::file_index(self.files.iter_mut());

                let file_scroll_view = Scrollable::new(&mut self.file_scrollable)
                    .width(Length::Fill)
                    .push(file_index);

                let content = if is_empty {
                    Column::new().push(Text::new("Drag and drop files here").color(Color::WHITE))
                } else {
                    Column::new()
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .push(file_scroll_view)
                };

                Container::new(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(styles::Container {
                        hovered: self.hovering_with_files,
                    })
                    .center_x()
                    .center_y()
                    .into()
            }
        }
    }
}
