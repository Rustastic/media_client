use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Cursor, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose, Engine};
use html_parser::{Dom, Node};
use image::{codecs::jpeg::JpegDecoder, DynamicImage};
use wg_2024::network::NodeId;

/// `(source_id, file_id)`
/// # Note
/// media file have not `source_id`
type FileKey = (Option<NodeId>, String);

type MediaContent = String;

pub enum AddedFileReturn {
    CompleteFile {
        source_id: NodeId,
        file_id: String,
        content: String,
        /// `(media_id, content)`
        media_content: HashMap<String, MediaContent>,
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
    ) -> Option<Vec<FileKey>> {
        let (text_file, media_ref) = TextFile::new_textfile(content, size);
        if media_ref.is_empty() {
            // display_file(AddedFileReturn::CompleteFile {
            //     source_id,
            //     file_id: file_id.to_owned(),
            //     content: text_file.content,
            //     media_content: HashMap::new(),
            // });
            return None;
        }
        self.files.insert(
            (Some(source_id), file_id.to_owned()),
            FileType::TextFile(text_file),
        );
        Some(media_ref)
    }
    pub fn add_media_file(&mut self, file_id: &str, content: String) -> Option<AddedFileReturn> {
        self.files
            .insert((None, file_id.to_owned()), FileType::MediaFile { content });
        if let Some(file) = self.check_and_take_complete_file() {
            display_file(file);
        }
        None
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
    MediaFile { content: MediaContent },
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
/// `<img src="media_id">`
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
                Some((None, element.attributes["src"].clone()?))
            }
            _ => None,
        })
        .collect::<Vec<FileKey>>();
    Some(media_ref)
}

fn display_file(file: AddedFileReturn) {
    if let AddedFileReturn::CompleteFile {
        source_id,
        file_id,
        content,
        media_content,
    } = file
    {
        let Ok(current_dir) = std::env::current_dir() else {
            return;
        };
        let a = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let dir_path = current_dir.join(format!("/browser/{source_id}_{file_id}_{a}",));
        let _ = fs::create_dir(&dir_path);
        let file_path = dir_path.join(file_id);

        if let Ok(mut text_file) = File::create(file_path.clone()) {
            let _ = write!(text_file, "{content}");
            let _ = text_file.flush();
            for (media_id, m_content) in media_content {
                if let Some(image) = get_dynimage_from_string(m_content) {
                    let _ = image.save(dir_path.join(media_id));
                };
            }
        }
        let _ = webbrowser::open(file_path.to_str().unwrap_or_default());
    }
}

fn get_dynimage_from_string(base_64: String) -> Option<DynamicImage> {
    let file_media_content = general_purpose::STANDARD.decode(base_64).ok()?;
    let cursor = Cursor::new(file_media_content);
    let decoder = JpegDecoder::new(cursor).ok()?;
    DynamicImage::from_decoder(decoder).ok()
}

// #[cfg(test)]
// #[test]
// fn test_display_file() {
//     use base64::{Engine as _, engine::general_purpose};
//     use image::ImageReader;

//     let text_content = std::fs::read_to_string(r"C:\__git\Servers\src\servers\text_files\file2.html").unwrap();
//     let file_media_content = ImageReader::open(r"C:\__git\Servers\src\servers\data_files\media2.jpg").unwrap().decode().unwrap();
//     let mut buf = Vec::new();
//     file_media_content.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Jpeg);
//     let base_64 = general_purpose::STANDARD.encode(&buf);

//     test_args(base_64);

//     // let mut media_content = HashMap::new();
//     // media_content.insert("media2.jpg".to_string(), file_media_content);
//     // let complete_file = AddedFileReturn::CompleteFile { source_id: 2, file_id: "text.html".to_string(), content: text_content, media_content };
//     // display_file(complete_file);
//     // println!("{:?}", std::env::current_dir());
// }

// fn test_args( base_64: String ) {
//     let file_media_content = general_purpose::STANDARD.decode(base_64).unwrap();
//     let cursor = Cursor::new(file_media_content);
//     let decoder = JpegDecoder::new(cursor) ;
//     let image = DynamicImage::from_decoder(decoder.unwrap()).unwrap();
//     image.save(r"C:\__git\media_client\0_text.html\media1.jpg");
// }
