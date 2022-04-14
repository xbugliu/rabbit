use std::time::Instant;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use walkdir::{WalkDir, DirEntry};
use anyhow::{Result, anyhow};
use mime::Mime;

pub mod index;
mod tantivy_jieba;
mod doc;


struct IndexStat {
    indexed_count: i32,
    failed_count: i32,
    start_time: Instant,
}

impl std::fmt::Display for IndexStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.indexed_count + self.failed_count;
        write!(f, "total doc={}, indexed doc={}, failed doc={}, cost time={}ms", total, self.indexed_count, self.failed_count, self.start_time.elapsed().as_millis())
    }
}

pub fn start_monitor(index: &index::IndexServer) {

}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}


pub fn recursive_index(index: &mut index::IndexServer, dir: &str) {

    let mut index_stat = IndexStat{indexed_count: 0, failed_count: 0, start_time: Instant::now()};

    let walker = WalkDir::new(dir).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry.as_ref().unwrap();
        if entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path().to_str().unwrap();
        let doc_mime = doc::is_document_file(path);
        if doc_mime.is_ok() {
            let doc_mime = &doc_mime.unwrap();
            let content = doc::convert_docment_to_plain_text(doc_mime, path);
            if content.is_ok() {
                log::info!("add doc {}, mime: {}", path, doc_mime.to_string());
                let mtime = entry.metadata().unwrap().modified().unwrap();
                let mtime = mtime
                                     .duration_since(UNIX_EPOCH)
                                     .expect("Time went backwards");
                let doc = doc::make_index_document(path, mtime.as_secs(),  doc_mime,content.unwrap());
                index.add_update_doc(doc.unwrap());
                index_stat.indexed_count += 1;
            }else {
                log::error!("convert file: {} err: {}", path, content.err().unwrap());
                index_stat.failed_count += 1;
            }
        }
    }

    log::info!("dir: {}, index stat: {}", dir, index_stat);

    index.commit();
}