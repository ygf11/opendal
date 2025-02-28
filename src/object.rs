// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use futures::future::BoxFuture;
use futures::ready;

use crate::error::Kind;
use crate::error::Result;
use crate::ops::OpDelete;
use crate::ops::OpList;
use crate::ops::OpStat;
use crate::Accessor;
use crate::Reader;
use crate::Writer;

/// Handler for all object related operations.
#[derive(Clone, Debug)]
pub struct Object {
    acc: Arc<dyn Accessor>,
    meta: Metadata,
}

impl Object {
    /// Creates a new Object.
    pub fn new(acc: Arc<dyn Accessor>, path: &str) -> Self {
        Self {
            acc,
            meta: Metadata {
                path: path.to_string(),
                ..Default::default()
            },
        }
    }

    /// Create a new reader which can read the whole object.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///
    ///     let bs = "Hello, World!".as_bytes().to_vec();
    ///     op.object("test").writer().write_bytes(bs).await?;
    ///
    ///     // Read whole file.
    ///     let mut r = op.object("test").reader();
    ///     io::copy(&mut r, &mut io::sink()).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn reader(&self) -> Reader {
        Reader::new(self.acc.clone(), self.meta.path(), None, None)
    }

    /// Create a new ranged reader which can only read data between [offset, offset+size).
    ///
    /// # Note
    ///
    /// The input offset and size are not checked, callers could meet error
    /// while reading.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///
    ///     let bs = "Hello, World!".as_bytes().to_vec();
    ///     op.object("test").writer().write_bytes(bs).await?;
    ///
    ///     // Read within [1, 2) bytes.
    ///     let mut r = op.object("test").range_reader(1, 1);
    ///     io::copy(&mut r, &mut io::sink()).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn range_reader(&self, offset: u64, size: u64) -> Reader {
        Reader::new(self.acc.clone(), self.meta.path(), Some(offset), Some(size))
    }

    /// Create a new offset reader which can read data since offset.
    ///
    /// # Note
    ///
    /// The input offset is not checked, callers could meet error while reading.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///
    ///     let bs = "Hello, World!".as_bytes().to_vec();
    ///     op.object("test").writer().write_bytes(bs).await?;
    ///
    ///     // Read start offset 4.
    ///     let mut r = op.object("test").offset_reader(4);
    ///     io::copy(&mut r, &mut io::sink()).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn offset_reader(&self, offset: u64) -> Reader {
        Reader::new(self.acc.clone(), self.meta.path(), Some(offset), None)
    }

    /// Create a new limited reader which can only read limited data.
    ///
    /// # Note
    ///
    /// The input size is not checked, callers could meet error while reading.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///
    ///     let bs = "Hello, World!".as_bytes().to_vec();
    ///     op.object("test").writer().write_bytes(bs).await?;
    ///
    ///     // Read within 8 bytes.
    ///     let mut r = op.object("test").limited_reader(8);
    ///     io::copy(&mut r, &mut io::sink()).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn limited_reader(&self, size: u64) -> Reader {
        Reader::new(self.acc.clone(), self.meta.path(), None, Some(size))
    }

    /// Create a new writer which can write data into the object.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///
    ///     let bs = "Hello, World!".as_bytes().to_vec();
    ///     op.object("test").writer().write_bytes(bs).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn writer(&self) -> Writer {
        Writer::new(self.acc.clone(), self.meta.path())
    }

    /// Delete current object.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///
    ///     let bs = "Hello, World!".as_bytes().to_vec();
    ///     op.object("test").delete().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete(&self) -> Result<()> {
        let op = &OpDelete::new(self.meta.path());

        self.acc.delete(op).await
    }

    /// Get current object's metadata.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    /// use opendal::error::Kind;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///
    ///     if let Err(e) =  op.object("test").metadata().await {
    ///         if e.kind() == Kind::ObjectNotExist {
    ///             println!("object not exist")
    ///         }
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn metadata(&self) -> Result<Metadata> {
        let op = &OpStat::new(self.meta.path());

        self.acc.stat(op).await
    }

    /// Use local cached metadata if possible.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    /// use opendal::error::Kind;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///     let mut o = op.object("test");
    ///
    ///     o.metadata_cached().await;
    ///     // The second call to metadata_cached will have no cost.
    ///     o.metadata_cached().await;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn metadata_cached(&mut self) -> Result<&Metadata> {
        if self.meta.complete() {
            return Ok(&self.meta);
        }

        let op = &OpStat::new(self.meta.path());
        self.meta = self.acc.stat(op).await?;

        Ok(&self.meta)
    }

    pub(crate) fn metadata_mut(&mut self) -> &mut Metadata {
        &mut self.meta
    }

    /// Check if this object exist or not.
    ///
    /// # Example
    ///
    /// ```
    /// use opendal::services::memory;
    /// use anyhow::Result;
    /// use futures::io;
    /// use opendal::Operator;
    /// use opendal::error::Kind;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let op = Operator::new(memory::Backend::build().finish().await?);
    ///     let _ = op.object("test").is_exist().await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn is_exist(&self) -> Result<bool> {
        let r = self.metadata().await;
        match r {
            Ok(_) => Ok(true),
            Err(err) => match err.kind() {
                Kind::ObjectNotExist => Ok(false),
                _ => Err(err),
            },
        }
    }
}

/// Metadata carries all object metadata.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    complete: bool,

    path: String,
    mode: Option<ObjectMode>,

    content_length: Option<u64>,
}

impl Metadata {
    /// Returns object path that relative to corresponding backend's root.
    pub fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn set_path(&mut self, path: &str) -> &mut Self {
        self.path = path.to_string();
        self
    }

    pub fn complete(&self) -> bool {
        self.complete
    }

    pub(crate) fn set_complete(&mut self) -> &mut Self {
        self.complete = true;
        self
    }

    pub fn mode(&self) -> ObjectMode {
        debug_assert!(self.mode.is_some(), "mode must exist");

        self.mode.unwrap_or_default()
    }

    pub(crate) fn set_mode(&mut self, mode: ObjectMode) -> &mut Self {
        self.mode = Some(mode);
        self
    }

    pub fn content_length(&self) -> u64 {
        debug_assert!(self.content_length.is_some(), "content length must exist");

        self.content_length.unwrap_or_default()
    }

    pub(crate) fn set_content_length(&mut self, content_length: u64) -> &mut Self {
        self.content_length = Some(content_length);
        self
    }
}

/// ObjectMode represents the corresponding object's mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ObjectMode {
    /// FILE means the object has data to read.
    FILE,
    /// DIR means the object can be listed.
    DIR,
    /// Unknown means we don't know what we can do on thi object.
    Unknown,
}

impl Default for ObjectMode {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Display for ObjectMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectMode::FILE => write!(f, "file"),
            ObjectMode::DIR => write!(f, "dir"),
            ObjectMode::Unknown => write!(f, "unknown"),
        }
    }
}

pub type BoxedObjectStream = Box<dyn futures::Stream<Item = Result<Object>> + Unpin + Send>;

/// Handler for listing object under a dir.
pub struct ObjectStream {
    acc: Arc<dyn Accessor>,
    path: String,
    state: State,
}

enum State {
    Idle,
    Sending(BoxFuture<'static, Result<BoxedObjectStream>>),
    Listing(BoxedObjectStream),
}

impl ObjectStream {
    /// Creates a new object stream.
    pub fn new(acc: Arc<dyn Accessor>, path: &str) -> Self {
        Self {
            acc,
            path: path.to_string(),
            state: State::Idle,
        }
    }
}

impl futures::Stream for ObjectStream {
    type Item = Result<Object>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &mut self.state {
            State::Idle => {
                let acc = self.acc.clone();
                let op = OpList::new(&self.path);

                let future = async move { acc.list(&op).await };

                self.state = State::Sending(Box::pin(future));
                self.poll_next(cx)
            }
            State::Sending(future) => match ready!(Pin::new(future).poll(cx)) {
                Ok(obs) => {
                    self.state = State::Listing(obs);
                    self.poll_next(cx)
                }
                Err(e) => Poll::Ready(Some(Err(e))),
            },
            State::Listing(obs) => Pin::new(obs).poll_next(cx),
        }
    }
}
