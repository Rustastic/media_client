use std::collections::HashMap;
use wg_2024::network::NodeId;

pub type FileId = String;
pub type MediaId = String;
pub type MediaContent = String;
pub struct TextFile {
    content: String,
    size: usize,
    media_ref: Vec<(NodeId, MediaId)>,
}
impl TextFile {
    fn new(content: String, size: usize) -> Self {
        let media_ref = vec![];
        //TODO: get_ref
        Self {
            content,
            media_ref,
            size,
        }
    }
    pub fn get_media_ref(&self) -> &Vec<(NodeId, MediaId)> {
        &self.media_ref
    }
    pub fn get_media_number(&self) -> usize {
        self.media_ref.len()
    }
    pub fn get_content(&self) -> String {
        self.content.clone()
    }
}

pub enum Server {
    TextServer(TextServer),
    MediaServer(MediaServer),
}

#[derive(Default)]
pub struct TextServer {
    text_files: HashMap<FileId, Option<TextFile>>,
}

impl TextServer {
    pub fn new() -> Self {
        Default::default()
    }
    /// # Note
    /// If a file was already present it will be deleted
    pub fn add_file_id(&mut self, id: FileId) {
        self.text_files.insert(id, None);
    }
    pub fn bulk_add_file_id(&mut self, ids: Vec<FileId>) {
        for file_id in ids {
            self.add_file_id(file_id)
        }
    }
    /// Add a file to the server
    ///
    /// # Returns
    /// A vector of `(NodeId, MediaId)` representing references to the media in the file content
    pub fn add_file(
        &mut self,
        id: FileId,
        file_content: String,
        size: usize,
    ) -> Vec<(NodeId, MediaId)> {
        let file = TextFile::new(file_content, size);
        self.text_files.insert(id.clone(), Some(file));
        self.get_file(&id).map_or(vec![], |f| f.media_ref.clone())
    }
    fn get_file(&self, id: &FileId) -> Option<&TextFile> {
        self.text_files.get(id)?.as_ref()
    }
    pub fn get_files(&self) -> &HashMap<FileId, Option<TextFile>> {
        &self.text_files
    }
}

#[derive(Default)]
pub struct MediaServer {
    media_files: HashMap<MediaId, Option<MediaContent>>,
}

impl MediaServer {
    pub fn new() -> Self {
        Default::default()
    }
    /// # Note
    /// If a media was already present it will be deleted
    pub fn add_media_id(&mut self, id: MediaId) {
        self.media_files.insert(id, None);
    }
    pub fn add_media(&mut self, id: MediaId, media_content: MediaContent) {
        self.media_files.insert(id, Some(media_content));
    }
    pub fn get_media(&self, id: &MediaId) -> Option<&MediaContent> {
        self.media_files.get(id)?.as_ref()
    }
}
