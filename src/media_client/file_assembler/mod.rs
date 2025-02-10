use std::{
    collections::HashMap,
    fs::{self, File},
    io::{stderr, stdout, Write},
};

use html_parser::{Dom, Node};
use wg_2024::network::NodeId;

/// `(source_id, file_id)`
/// # Note
/// media file have not `source_id`
type FileKey = (Option<NodeId>, String);

type MediaContent = Vec<u8>;

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
    pub fn add_media_file(&mut self, file_id: &str, content: Vec<u8>) -> Option<AddedFileReturn> {
        self.files
            .insert((None, file_id.to_owned()), FileType::MediaFile { content });
        if let Some(file) = self.check_and_take_complete_file() {
            // display_file(file);
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
    MediaFile { content: Vec<u8> },
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

// fn display_file(file: AddedFileReturn) {
//     if let AddedFileReturn::CompleteFile {
//         source_id,
//         file_id,
//         content,
//         media_content,
//     } = file
//     {
//         let _join = std::thread::spawn(move || {
//             stdout().flush();
//             stderr().flush();
//             let Ok(current_dir) = std::env::current_dir() else {
//                 return;
//             };
//             // let a = Instant::now();
//             let dir_path =  current_dir.join(format!("{source_id}_{file_id}"));
//             println!("dir_path: {}", dir_path.display());
//             stdout().flush();
//             let _ = fs::create_dir(&dir_path);
//             let file_path = dir_path.join(file_id);
//             println!("file_path: {}", file_path.display());

//             if let Ok(mut text_file) = File::create(file_path.clone()).inspect_err(|e|{
//                 println!("error: {e}");
//             }) {
//                 let error = write!(text_file, "{content}");
//                 let _ = text_file.flush();
//                 println!("error: {error:?} \n");
//                 stdout().flush();
//                 for (media_id, m_content) in media_content {
//                     let dynimage = image::load_from_memory(&m_content).unwrap();
//                     let a = Image::load_from_memory;
//                     dynimage.save(dir_path.join(media_id));
//                 }
//             } else {
//                 println!("error_creating text file");
//             };
//             while webbrowser::open(file_path.to_str().unwrap()).is_ok() {
//             }
            
//         });
//     }
// }


// #[cfg(test)]
// #[test]
// fn test_display_file() {
//     use image::ImageReader;
    
//     let text_content = std::fs::read_to_string(r"C:\__git\Servers\src\servers\text_files\file2.html").unwrap();
//     let file_media_content = ImageReader::open(r"C:\__git\Servers\src\servers\data_files\media2.jpg").unwrap().decode().unwrap().into_bytes();
//     let dynimage = image::load_from_memory(&file_media_content).unwrap();
//     let mut media_content = HashMap::new();
//     media_content.insert("media2.jpg".to_string(), file_media_content);
//     let complete_file = AddedFileReturn::CompleteFile { source_id: 2, file_id: "text.html".to_string(), content: text_content, media_content };
//     display_file(complete_file);
//     // println!("{:?}", std::env::current_dir());
// }