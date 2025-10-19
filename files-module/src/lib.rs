use std::io::{Read, Seek};

use docx_rust::{
    DocxFile, DocxResult,
    document::{BodyContent, ParagraphContent, RunContent},
};

pub mod app;

pub const RUYI_FILES: &str = "ruyi-files";

pub fn extract_text_docx(reader: impl Read + Seek) -> DocxResult<String> {
    let docx_file = DocxFile::from_reader(reader)?;
    let docx = docx_file.parse()?;
    let mut all_text = String::new();

    for child in docx.document.body.content {
        if let BodyContent::Paragraph(para) = child {
            for child in para.content {
                match child {
                    ParagraphContent::Run(run) => {
                        for child in run.content {
                            if let RunContent::Text(text) = child {
                                all_text.push_str(&text.text);
                            }
                        }
                    }
                    ParagraphContent::Link(link) => {
                        if let Some(run) = link.content {
                            for child in run.content {
                                if let RunContent::Text(text) = child {
                                    all_text.push_str(&text.text);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            all_text.push('\n');
        }
    }

    Ok(all_text)
}

pub fn extract_text_pdf(buffer: &[u8]) -> Result<String, pdf_extract::OutputError> {
    pdf_extract::extract_text_from_mem(buffer)
}
