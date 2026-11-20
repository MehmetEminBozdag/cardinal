//! On-disk snapshot of the size tree (postcard + zstd, written atomically).

use crate::tree::SizeTree;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

const MAGIC: &[u8; 8] = b"FSIZE\0\0\x01";

#[derive(Serialize, Deserialize)]
struct Snapshot {
    tree: SizeTree,
}

pub fn write(file: &Path, tree: &SizeTree) -> Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).context("create snapshot directory")?;
    }
    let tmp = file.with_extension("tmp");
    {
        let out = File::create(&tmp).context("create snapshot file")?;
        let mut out = BufWriter::new(out);
        out.write_all(MAGIC)?;
        let mut enc = zstd::Encoder::new(out, 3)?;
        let bytes = postcard::to_stdvec(&Snapshot { tree: tree.clone() })?;
        enc.write_all(&bytes)?;
        enc.finish()?.flush()?;
    }
    std::fs::rename(&tmp, file).context("replace snapshot file")?;
    Ok(())
}

pub fn read(file: &Path) -> Result<SizeTree> {
    let mut input = BufReader::new(File::open(file).context("open snapshot")?);
    let mut magic = [0u8; 8];
    input.read_exact(&mut magic)?;
    if &magic != MAGIC {
        bail!("unknown folder-size snapshot format");
    }
    let mut bytes = Vec::new();
    zstd::Decoder::new(input)?.read_to_end(&mut bytes)?;
    let snapshot: Snapshot = postcard::from_bytes(&bytes).context("decode snapshot")?;
    Ok(snapshot.tree)
}
