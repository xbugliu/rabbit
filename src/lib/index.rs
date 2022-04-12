use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema;
use tantivy::{doc, Index, ReloadPolicy, TantivyError, IndexWriter};
use anyhow::Result;
use crate::doc::Document;

pub struct IndexServer {
    index: Index,
    writer: IndexWriter,
}

impl IndexServer {
    pub fn new(dir: &str) -> Result<IndexServer>  {
        let mut schema_builder = schema::Schema::builder();
        let schema = schema_builder.build();
    
        let index =
        Index::create_in_dir(dir, schema.clone()).or_else(|error| match error {
            TantivyError::IndexAlreadyExists => Ok(Index::open_in_dir(dir)?),
            _ => Err(error),
        })?;
    
        let mut writer = index.writer(50_000_000)?;
    
        let service = IndexServer{index, writer};
        Ok(service)
    }

    pub fn search(self) {

    }

    pub fn add_update_doc(&mut self, doc: Document) {
        let mut d = tantivy::schema::Document::default();
        self.writer.add_document(d);
    }

    pub fn del_doc(self) {

    }

    pub fn commit(&mut self) {
        self.writer.commit();
    }
}