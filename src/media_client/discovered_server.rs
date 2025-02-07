use std::collections::HashMap;

use messages::high_level_messages::ServerType;
use wg_2024::network::NodeId;

type FileId = String;
type MediaId = String;
type FileContent = String;
type MediaContent = String;

struct DiscoveredServer{
    id: NodeId,
    server_type: Option<ServerType>,

    files_ids: Option<Vec<FileId>>,
    files: Vec<ConstructedFile>,
}

struct ConstructedFile {
    file_id: FileId,
    file: FileContent,
    media_id: Option<HashMap<NodeId, MediaId>>,
    media: Option<HashMap<(NodeId, MediaId), MediaContent>>
}

impl ConstructedFile {
    fn new(file_id: FileId, file: FileContent) -> Self {
        //TODO parse file and get media_id
        let media_id =; //TODO;

        Self { file_id, file, media_id, media: None }
    }
}