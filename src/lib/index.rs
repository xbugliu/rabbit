use std::time::Instant;
use tantivy::collector::TopDocs;
use tantivy::query::{QueryParser, TermQuery};
use tantivy::schema::{Schema, TEXT, FAST, STORED, STRING, INDEXED, Field, TextOptions, IndexRecordOption, TextFieldIndexing};
use tantivy::{Index, ReloadPolicy, TantivyError, IndexWriter, Term};
use anyhow::Result;
use log::{debug, error, info, warn};
use crate::doc::Document;
use crate::tantivy_jieba;

pub struct IndexServer {
    index: Index,
    writer: IndexWriter,
    schema: Schema,
}

pub struct  SearchResult {
    pub paths: Vec<String>
}

impl IndexServer {
    pub fn new(dir: &str) -> Result<IndexServer>  {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("filename", STRING | STORED);
        schema_builder.add_u64_field("signature", INDEXED);

        let text_field_indexing = TextFieldIndexing::default()
            .set_tokenizer("jie_ba")
            .set_index_option(IndexRecordOption::Basic);
        let text_options = TextOptions::default()
            .set_indexing_options(text_field_indexing);

        schema_builder.add_text_field("body", text_options);
        schema_builder.add_u64_field("mtime", STORED);
        schema_builder.add_u64_field("mime_type", FAST | STORED);

        let schema = schema_builder.build();
    
        let index =
        Index::create_in_dir(dir, schema.clone()).or_else(|error| match error {
            TantivyError::IndexAlreadyExists => Ok(Index::open_in_dir(dir)?),
            _ => Err(error),
        })?;

        let tokenizer = tantivy_jieba::JiebaTokenizer{};
        index.tokenizers().register("jie_ba", tokenizer);
    
        let writer = index.writer(50_000_000)?;
    
        let service = IndexServer{index, writer, schema};
        Ok(service)
    }

    pub fn search(&self, querycontent: String) -> Result<SearchResult>{
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let query_parser = QueryParser::for_index(&self.index, vec![self.get_field("body")]);
        let now = Instant::now();
        let query = query_parser.parse_query(&querycontent)?;
        info!("parse_query cost={}", now.elapsed().as_millis());
        
        let searcher = reader.searcher();

        let now = Instant::now();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(100))?;
        info!("search cost={}", now.elapsed().as_millis());
        let now = Instant::now();

        let mut paths = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            let path = retrieved_doc.get_first(self.get_field("filename")).unwrap();
            paths.push(String::from(path.as_text().unwrap()))
        };
        info!("load docs cost={}", now.elapsed().as_millis());

        Ok(SearchResult{
            paths: paths
        })

    }

    fn get_field(&self, field_name: &str) -> Field {
        self.schema.get_field(field_name).expect(&format!("filed {} must exist", field_name))
    }

    pub fn add_update_doc(&mut self, doc: Document)  { 

        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into();

        if reader.is_err() {
            let err = reader.err().unwrap();
            log::error!("add_update_doc err: {}", err.to_string());
            return;
        }
        let reader = reader.unwrap();
        let searcher = reader.searcher();
        let query = TermQuery::new(
            Term::from_field_u64(self.schema.get_field("signature").unwrap(), doc.signature),
            IndexRecordOption::Basic,
        );
        let top_docs = searcher.search(&query, &TopDocs::with_limit(1)).unwrap();
        if top_docs.len() > 0 {
            return;
        }

        let mut d = tantivy::schema::Document::default();
        d.add_field_value(self.get_field("filename"), doc.filename.as_str());
        d.add_field_value(self.get_field("signature"), doc.signature);
        d.add_field_value(self.get_field("body"), doc.body);
        d.add_field_value(self.get_field("mtime"), doc.mtime);
        d.add_field_value(self.get_field("mime_type"), doc.mime_type);
        
        let result = self.writer.add_document(d);
        match result {
            Err(e) => log::error!("add doc err: {}", e),
            _ => log::info!("add doc success {}", doc.filename)
        };
    }

    pub fn del_doc(&mut self, doc_signature: u64) {
        let term = Term::from_field_u64(self.schema.get_field("signature").unwrap(), doc_signature);
        self.writer.delete_term(term);
        self.writer.commit();
    }

    pub fn commit(&mut self) {
        self.writer.commit();
    }
}