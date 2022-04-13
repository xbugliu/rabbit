//! A library that bridges between tantivy and jieba-rs.
//!
//! It implements a [`JiebaTokenizer`](./struct.JiebaTokenizer.html) for the purpose.
#![forbid(unsafe_code)]

extern crate jieba_rs;
extern crate tantivy;
use lazy_static::lazy_static;

use tantivy::tokenizer::{BoxTokenStream, Token, TokenStream, Tokenizer};

lazy_static! {
    static ref JIEBA: jieba_rs::Jieba = jieba_rs::Jieba::new();
}

/// Tokenize the text using jieba_rs.
///
/// Need to load dict on first tokenization.
///
/// # Example
/// ```rust
/// use tantivy::tokenizer::*;
/// let tokenizer = tantivy_jieba::JiebaTokenizer {};
/// let mut token_stream = tokenizer.token_stream("测试");
/// assert_eq!(token_stream.next().unwrap().text, "测试");
/// assert!(token_stream.next().is_none());
/// ```
///
/// # Register tantivy tokenizer
/// ```rust
/// use tantivy::schema::Schema;
/// use tantivy::tokenizer::*;
/// use tantivy::Index;
/// # fn main() {
/// # let schema = Schema::builder().build();
/// let tokenizer = tantivy_jieba::JiebaTokenizer {};
/// let index = Index::create_in_ram(schema);
/// index.tokenizers()
///      .register("jieba", tokenizer);
/// # }
#[derive(Clone)]
pub struct JiebaTokenizer;

/// Token stream instantiated by [`JiebaTokenizer`](./struct.JiebaTokenizer.html).
///
/// Use [`JiebaTokenizer::token_stream`](./struct.JiebaTokenizer.html#impl-Tokenizer<%27a>).
pub struct JiebaTokenStream {
    tokens: Vec<Token>,
    index: usize,
}

impl TokenStream for JiebaTokenStream {
    fn advance(&mut self) -> bool {
        if self.index < self.tokens.len() {
            self.index = self.index + 1;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.index - 1]
    }
}

impl Tokenizer for JiebaTokenizer {
    fn token_stream<'a>(&self, text: &'a str) -> BoxTokenStream<'a> {
        let mut indices = text.char_indices().collect::<Vec<_>>();
        indices.push((text.len(), '\0'));
        let orig_tokens = JIEBA.tokenize(text, jieba_rs::TokenizeMode::Search, true);
        let mut tokens = Vec::new();
        for i in 0..orig_tokens.len() {
            let token = &orig_tokens[i];
            tokens.push(Token {
                offset_from: indices[token.start].0,
                offset_to: indices[token.end].0,
                position: token.start,
                text: String::from(&text[(indices[token.start].0)..(indices[token.end].0)]),
                position_length: token.end - token.start,
            });
        }
        BoxTokenStream::from(JiebaTokenStream { tokens, index: 0 })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use tantivy::tokenizer::*;
        let tokenizer = crate::tantivy_jieba::JiebaTokenizer {};
        let mut token_stream = tokenizer.token_stream(
            "张华考上了北京大学；李萍进了中等技术学校；我在百货公司当售货员：我们都有光明的前途",
        );
        let mut tokens = Vec::new();
        let mut token_text = Vec::new();
        while let Some(token) = token_stream.next() {
            tokens.push(token.clone());
            token_text.push(token.text.clone());
        }
        // offset should be byte-indexed
        assert_eq!(tokens[0].offset_from, 0);
        assert_eq!(tokens[0].offset_to, "张华".bytes().len());
        assert_eq!(tokens[1].offset_from, "张华".bytes().len());
        // check tokenized text
        assert_eq!(
            token_text,
            vec![
                "张华",
                "考上",
                "了",
                "北京",
                "大学",
                "北京大学",
                "；",
                "李萍",
                "进",
                "了",
                "中等",
                "技术",
                "术学",
                "学校",
                "技术学校",
                "；",
                "我",
                "在",
                "百货",
                "公司",
                "百货公司",
                "当",
                "售货",
                "货员",
                "售货员",
                "：",
                "我们",
                "都",
                "有",
                "光明",
                "的",
                "前途"
            ]
        );
    }
}
