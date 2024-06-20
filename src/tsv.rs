use std::{collections::HashMap, fs, ops::Range, path::Path};

use crate::errors::{Error, Result};

pub struct Tsv {
    _header_fields: Vec<String>,
    header_to_index: HashMap<String, usize>,
    rows: Vec<Vec<String>>,
    path: String,
}

#[derive(Clone, Copy)]
pub struct TsvRow<'a> {
    index: usize,
    tsv: &'a Tsv,
}

pub struct TsvIterator<'a> {
    index_iterator: Range<usize>,
    tsv: &'a Tsv,
}

impl<'a> Iterator for TsvIterator<'a> {
    type Item = TsvRow<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index_iterator.next()?;
        Some(TsvRow {
            index,
            tsv: self.tsv,
        })
    }
}

impl<'a> IntoIterator for &'a Tsv {
    type Item = TsvRow<'a>;
    type IntoIter = TsvIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TsvIterator {
            index_iterator: 0..self.rows.len(),
            tsv: self,
        }
    }
}

impl<'a> TsvRow<'a> {
    pub fn get(&'a self, field: &str) -> Result<&'a str> {
        // This isn't super fast.. but because it's just used
        // during the input phase that doesn't matter much.
        let index = *self
            .tsv
            .header_to_index
            .get(field)
            .ok_or_else(|| Error::MissingTsvField {
                tsv_path: String::from(&self.tsv.path),
                field: field.into(),
            })?;

        Ok(&self.tsv.rows[self.index][index])
    }
}

fn split_line(line: &str) -> Vec<String> {
    line.split('\t').map(String::from).collect()
}

impl Tsv {
    pub fn read_from_path(path: &Path) -> Result<Self> {
        let path_lossy = path.to_string_lossy();

        let file_contents =
            fs::read_to_string(path).map_err(|io_error| Error::FailedToReadTsv {
                tsv_path: (&*path_lossy).into(),
                io_error,
            })?;

        Self::try_from_str(&path_lossy, &file_contents)
    }

    pub fn try_from_str(path: &str, value: &str) -> Result<Self> {
        let mut lines_iter = value.lines();
        let header = lines_iter.next().unwrap_or_default();
        let header_fields = split_line(header);

        let header_to_index = header_fields
            .iter()
            .enumerate()
            .map(|(idx, field)| (field.clone(), idx))
            .collect();

        let rows = lines_iter
            .map(|line| {
                let fields = split_line(line);
                if fields.len() == header_fields.len() {
                    Ok(fields)
                } else {
                    Err(Box::new(Error::TsvNumFieldsMismatch {
                        path: path.into(),
                        header_len: header_fields.len(),
                        row_len: fields.len(),
                        row: line.into(),
                    }))
                }
            })
            .collect::<Result<_>>()?;

        Ok(Tsv {
            _header_fields: header_fields,
            rows,
            header_to_index,
            path: path.into(),
        })
    }
}
