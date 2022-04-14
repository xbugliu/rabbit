use std::time::{Instant, Duration};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;

use doc::Document;
use walkdir::{WalkDir, DirEntry};
use anyhow::{Result, anyhow};
use mime::Mime;
use threadpool::ThreadPool;

pub mod index;
mod tantivy_jieba;
mod doc;


struct IndexStat {
    indexed_count: AtomicU32,
    total_count: AtomicU32,
    start_time: Instant,
}

impl std::fmt::Display for IndexStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.total_count.load(Ordering::Relaxed);
        let failed_count = total - self.indexed_count.load(Ordering::Relaxed);
        let index_count = self.indexed_count.load(Ordering::Relaxed);
        write!(f, "total doc={}, indexed doc={}, failed doc={}, cost time={}ms", total, index_count, failed_count, self.start_time.elapsed().as_millis())
    }
}

pub fn start_monitor(index: index::IndexServer) {

}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}

fn make_doc(path: String, doc_mime: &Mime, mtime_secs: u64) -> Result<Document> {
    let content = doc::convert_docment_to_plain_text(doc_mime, &path);
    if content.is_ok() {
        log::info!("add doc {}, mime: {}", path, doc_mime.to_string());
        doc::make_index_document(&path, mtime_secs,  doc_mime,content.unwrap())
    }else {
        log::error!("convert file: {} err: {}", path, content.err().unwrap());
        Err(anyhow!("convert error"))
    }
}

fn get_mtime_secs(entry: &DirEntry) -> u64 {
    let metadata = entry.metadata().unwrap();
    let mtime = metadata.modified().unwrap();
    mtime.duration_since(UNIX_EPOCH)
                            .expect("Time went backwards").as_secs()
}

pub fn recursive_index(index: index::IndexServer, dir: &str) {

    let index_stat = IndexStat{indexed_count: AtomicU32::new(0), total_count: AtomicU32::new(0), start_time: Instant::now()};
    let thread_pool = ThreadPool::new(8);
    let (tx, rx) = channel();
    let mut index = index;
    let index_reader = index.get_view();

    let child = thread::spawn(move || {
        let mut index_count = 0;
        loop {
            let doc = rx.recv();
            match doc {
                Ok(doc) => { 
                    index.add_update_doc(doc);
                    index_count += 1;
                },
                Err(_) => return (index, index_count),
            }
        };
    });

    let walker = WalkDir::new(dir).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry.as_ref().unwrap();
        if entry.file_type().is_dir() {
            continue;
        }
        
        let path = String::from(entry.path().to_str().unwrap());
        let mtime_secs = get_mtime_secs(entry);
        if index_reader.check_doc_exist_by_signature(doc::get_docment_signature(&path, mtime_secs)).is_ok() {
            log::info!("doc already indexed: {}", path);
            continue;
        }
        let doc_mime = doc::is_document_file(&path);
        if doc_mime.is_ok() {
            index_stat.total_count.fetch_add(1, Ordering::Relaxed);
            let tx = tx.clone();
            thread_pool.execute(move||{
                let doc = make_doc(path, &doc_mime.unwrap(), mtime_secs);
                if doc.is_ok() {
                    tx.send(doc.unwrap());
                }
            })
        }
    }

    drop(tx);
    let mut index_count = 0;
    (index, index_count) = child.join().unwrap();
    index_stat.indexed_count.store(index_count, Ordering::Relaxed);

    log::info!("dir: {}, index stat: {}", dir, index_stat);

    index.commit();
}