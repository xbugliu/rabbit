use anyhow::{Result, anyhow};
use mime::Mime;
use xxhash_rust::const_xxh3::xxh3_64 as const_xxh3;

pub struct Document {
    pub filename: String,
    pub id: u64,         // to get doc content, hash_64(filename)
    pub signature: u64,  // to check file is not modify, hash_64(filename+mtime)
    pub body: String,
    pub mtime: u64,
    pub mime_type: u64,
}

pub fn is_document_file(path: &str) -> Result<Mime> {
    let guess = mime_guess::from_path(path).first();
    let guess = match guess {
        None => return Err(anyhow!("guess error")),
        Some(val) => val
    };


    match (guess.type_(), guess.subtype()) {
        (mime::TEXT, _) => return Ok(guess),
        _ => return Err(anyhow!("not text"))
    };
}

pub fn convert_docment_to_plain_text(mime: &Mime, path: &str) -> Result<String> {
    if mime.subtype() ==  mime::PLAIN {
        let result = std::fs::read_to_string(path);
        if result.is_err() {
            return Err(anyhow!(result.err().unwrap()))
        }
        return Ok(result.unwrap())
    }

    log::info!("convert_docment_to_plain_text file={}", path);
    let mut pandoc = pandoc::new();
    pandoc.add_input(path);
    pandoc.set_output(pandoc::OutputKind::Pipe);
    let output = pandoc.execute().unwrap();
    match output {
        pandoc::PandocOutput::ToBuffer(result) => return Ok(result),
        _ => panic!("convert_docment_to_plain_text")
    }
}

pub fn make_index_document(filename: &str, mtime: u64, mime: &Mime, content: String) -> Result<Document> {
    Ok(Document{
        filename: String::from(filename),
        signature: get_docment_signature(filename, mtime),
        id: const_xxh3(filename.as_bytes()),
        body: content,
        mtime: mtime,
        mime_type: 1,
    })
}

pub fn get_docment_signature(filename: &str, mtime: u64) -> u64 {
    let signature_context = format!("{}_{}", filename, mtime);
    const_xxh3(signature_context.as_bytes())
}