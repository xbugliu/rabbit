pub mod index;
mod tantivy_jieba;
mod doc;

use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::{WalkDir, DirEntry};
use anyhow::{Result, anyhow};
use mime::Mime;


pub fn start_monitor(index: &index::IndexServer) {

}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}


pub fn recursive_index(index: &mut index::IndexServer, dir: &str) {

    let walker = WalkDir::new(dir).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {

        let entry = entry.as_ref().unwrap();
        let path = entry.path().to_str().unwrap();
        let doc_mime = doc::is_document_file(path);
        if doc_mime.is_ok() {
            let doc_mime = &doc_mime.unwrap();
            let content = doc::convert_docment_to_plain_text(doc_mime, path);
            if content.is_ok() {
                let mtime = entry.metadata().unwrap().modified().unwrap();
                let mtime = mtime
                                     .duration_since(UNIX_EPOCH)
                                     .expect("Time went backwards");
                let doc = doc::make_index_document(path, mtime.as_secs(),  doc_mime,content.unwrap());
                index.add_update_doc(doc.unwrap());
            }
        }
    }

    index.commit();
}