use std::collections::HashMap;

use html_parser::{Dom, Node};
use wg_2024::network::NodeId;

/// `(source_id, file_id)`
/// # Note
/// media file have not `source_id`
type FileKey = (Option<NodeId>, String);

pub enum AddedFileReturn {
    CompleteFile {
        source_id: NodeId,
        file_id: String,
        content: String,
        /// `(media_id, content)`
        media_content: HashMap<String, String>,
    },
    RefToMedia(Vec<FileKey>),
}

#[derive(Default)]
pub struct FileAssembler {
    files: HashMap<FileKey, FileType>,
}

impl FileAssembler {
    pub fn new() -> Self {
        FileAssembler::default()
    }
    pub fn add_textfile(
        &mut self,
        source_id: NodeId,
        file_id: &str,
        content: String,
        size: usize,
    ) -> AddedFileReturn {
        let (text_file, media_ref) = TextFile::new_textfile(content, size);
        if media_ref.is_empty() {
            return AddedFileReturn::CompleteFile {
                source_id,
                file_id: file_id.to_owned(),
                content: text_file.content,
                media_content: HashMap::new(),
            };
        }
        self.files.insert(
            (Some(source_id), file_id.to_owned()),
            FileType::TextFile(text_file),
        );
        AddedFileReturn::RefToMedia(media_ref)
    }
    pub fn add_media_file(&mut self, file_id: &str, content: String) -> Option<AddedFileReturn> {
        self.files
            .insert((None, file_id.to_owned()), FileType::MediaFile { content });
        self.check_and_take_complete_file()
    }
    fn check_and_take_complete_file(&mut self) -> Option<AddedFileReturn> {
        let mut found_text_key = None;
        for (text_key, text_file) in self.text_files() {
            if let FileType::TextFile(text_file) = text_file {
                let mut completed = true;
                for media_ref in &text_file.media_ref {
                    if !self.contains_media_file(None, &media_ref.1) {
                        completed = false;
                        break;
                    }
                }
                if completed {
                    found_text_key = Some(text_key.clone());
                    break;
                }
            }
        }
        if let Some(text_key) = found_text_key {
            return self.take_complete_file(text_key.0, &text_key.1);
        }
        None
    }
    fn take_complete_file(
        &mut self,
        source_id: Option<NodeId>,
        file_id: &str,
    ) -> Option<AddedFileReturn> {
        let text_file = self.take_file(source_id, file_id)?;
        if let FileType::TextFile(text_file) = text_file {
            let mut media_content = HashMap::new();
            for media_ref in text_file.media_ref {
                let media_file = self.take_file(media_ref.0, &media_ref.1.clone());
                if let Some(FileType::MediaFile { content }) = media_file {
                    media_content.insert(media_ref.1, content);
                }
            }
            return Some(AddedFileReturn::CompleteFile {
                source_id: source_id.unwrap_or_default(),
                file_id: file_id.to_owned(),
                content: text_file.content,
                media_content,
            });
        }
        None
    }
    fn contains_media_file(&self, source_id: Option<NodeId>, media_id: &str) -> bool {
        self.files.contains_key(&(source_id, media_id.to_owned()))
    }
    fn take_file(&mut self, source_id: Option<NodeId>, file_id: &str) -> Option<FileType> {
        self.files.remove(&(source_id, file_id.to_owned()))
    }
    fn text_files(&self) -> HashMap<&FileKey, &FileType> {
        self.files
            .iter()
            .filter(|(_, v)| match v {
                FileType::TextFile(_) => true,
                FileType::MediaFile { content: _ } => false,
            })
            .collect::<HashMap<&FileKey, &FileType>>()
    }
}

enum FileType {
    TextFile(TextFile),
    MediaFile { content: String },
}

struct TextFile {
    content: String,
    size: usize,
    media_ref: Vec<FileKey>,
}
impl TextFile {
    /// # Returns
    /// a tuple containings the new `TextFile` instance and a vec with the `media_id` that need to be fetched
    fn new_textfile(content: String, size: usize) -> (Self, Vec<FileKey>) {
        let media_ref = search_ref(&content).unwrap_or_default();
        (
            Self {
                content,
                size,
                media_ref: media_ref.clone(),
            },
            media_ref,
        )
    }
}

///get `media_ref` from
/// `<img href="media_id">`
///
/// # Return
/// An optional vec of `(None, media_id)`
fn search_ref(file: &str) -> Option<Vec<FileKey>> {
    let media_ref = Dom::parse(file)
        .ok()?
        .children
        .first()?
        .into_iter()
        .filter_map(|item| match item {
            Node::Element(ref element) if element.name == "img" => {
                Some((None, element.attributes["media_id"].clone()?))
            }
            _ => None,
        })
        .collect::<Vec<FileKey>>();
    Some(media_ref)
}
