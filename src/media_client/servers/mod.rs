use std::collections::HashMap;

use messages::high_level_messages::ServerType;
use utils::{FileId, MediaContent, MediaId, MediaServer, Server, TextServer};
use wg_2024::network::NodeId;


mod utils;

struct CompleteFile {
    file_id: FileId,
    text_content: String,
    media_number: usize,
    media_index: usize,
    media_files: Vec<MediaContent>,
}
impl CompleteFile {
    fn new_building(file_id: FileId, text_file: String, media_number: usize) -> Self {
        Self { file_id, text_content: text_file, media_number, media_index: 0, media_files: vec![] }
    }
    fn new_without_media(file_id: FileId, text_file: String) -> Self {
        Self {
            file_id,
            text_content: text_file,
            media_number: 0,
            media_index: 0,
            media_files: vec![],
        }
    }
    fn add_media(&mut self, media_content: MediaContent) -> bool {
        self.media_files.push(media_content);
        self.media_index += 1;
        if self.media_index >= self.media_number {
            return true;
        }
        false
    }
}

#[derive(Default)]
pub struct KnownServers {
    servers: HashMap<NodeId, Option<Server>>,
}

impl KnownServers {
    pub fn new() -> Self {
        Default::default()
    }
    fn add_server_id(&mut self, id: NodeId) {
        self.servers.insert(id, None);
    }
    fn add_server_type(&mut self, id: NodeId, server_type: ServerType) {
        match server_type {
            ServerType::Chat => return,
            ServerType::Text => {
                self.servers
                    .insert(id, Some(Server::TextServer(TextServer::new())));
            }
            ServerType::Media => {
                self.servers
                    .insert(id, Some(Server::MediaServer(MediaServer::new())));
            }
        }
    }
    fn add_files_list(&mut self, files_ids: Vec<FileId>, server_id: NodeId) {
        if let Some(server) = self.get_text_server_mut(&server_id) {
            server.bulk_add_file_id(files_ids);
        };
    }
    /// # Returns
    /// - `Ok(complete_file)` : if the file has not references to media (with field
    ///     `media_files` set to `vec![]`)
    /// - `Err(media_ref)` : if the file has media to be fetched
    /// - `Err(vec![])` : otherwise
    fn add_text_file(
        &mut self,
        size: usize,
        content: String,
        file_id: FileId,
        source_id: NodeId,
    ) -> Result<CompleteFile, Vec<(NodeId, MediaId)>> {
        if let Some(text_server) = self.get_text_server_mut(&source_id) {
            let media_ref = text_server.add_file(file_id.clone(), content.clone(), size);
            if media_ref.is_empty() {
                return Ok(CompleteFile::new_without_media(file_id, content));
            }
            for (node_id, media_id) in &media_ref {
                if let Some(media_server) = self.get_media_server_mut(&node_id) {
                    media_server.add_media_id(media_id.to_string());
                }
            }
            return Err(media_ref);
        };
        Err(vec![])
    }
    fn add_media_file(
        &mut self,
        media_id: MediaId,
        content: MediaContent,
        source_id: NodeId,
    ) -> Option<CompleteFile> {
        let media_server = self.get_media_server_mut(&source_id)?;
        media_server.add_media(media_id, content);
        self.check_for_complete_file()
    }
    fn check_for_complete_file(&mut self) -> Option<CompleteFile> {
        for (_, text_server) in self.get_text_servers() {
            for (text_id, file) in text_server.get_files() {
                if let Some(file) = file {
                    let mut builded_file = CompleteFile::new_building(text_id.clone(), file.get_content(), file.get_media_number());
                    for (media_server_id, media_id) in file.get_media_ref() {
                        if let Some(media_content) = self.get_media_file(*media_server_id, media_id.to_string()) {
                            if builded_file.add_media(media_content) {
                                return Some(builded_file)
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

impl KnownServers {
    // getter/setter
    fn get_media_file(&self, node_id: NodeId, media_id: MediaId) -> Option<MediaContent> {
        self.get_media_server(&node_id)?.get_media(&media_id).cloned()
    }
    fn get_text_server_mut(&mut self, id: &NodeId) -> Option<&mut TextServer> {
        self.servers
            .get_mut(id)?
            .as_mut()
            .and_then(|server| match server {
                Server::TextServer(text_server) => Some(text_server),
                Server::MediaServer(_) => None,
            })
    }
    fn get_media_server_mut(&mut self, id: &NodeId) -> Option<&mut MediaServer> {
        self.servers
            .get_mut(id)?
            .as_mut()
            .and_then(|server| match server {
                Server::MediaServer(media_server) => Some(media_server),
                Server::TextServer(_) => None,
            })
    }
    fn get_media_server(&self, id: &NodeId) -> Option<&MediaServer> {
        self.servers
            .get(id)?
            .as_ref()
            .and_then(|server| match server {
                Server::MediaServer(media_server) => Some(media_server),
                Server::TextServer(_) => None,
            })
    }
    fn get_text_servers(&self) -> Vec<(NodeId, &TextServer)> {
        self
            .servers
            .iter()
            .filter_map(|(&id, server)|{
                match server.as_ref()?{
                    Server::TextServer(ref  text_server) => Some((id, text_server)),
                    Server::MediaServer(_) => todo!(),
                }
            })
            .collect()
    }
}
