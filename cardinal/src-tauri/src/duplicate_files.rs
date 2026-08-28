use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

pub(crate) const MAX_DUPLICATE_CANDIDATES: usize = 20_000;
pub(crate) const MAX_DUPLICATE_LOGICAL_BYTES: u64 = 50 * 1024 * 1024 * 1024;
const MAX_DUPLICATE_READ_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const PARTIAL_HASH_BYTES: usize = 64 * 1024;
static DUPLICATE_SCAN_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(crate) struct DuplicateInput {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DuplicateScanResponse {
    pub scanned_files: usize,
    pub skipped_files: usize,
    pub limited: bool,
    pub cancelled: bool,
    pub groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DuplicateGroup {
    pub size: u64,
    pub files: Vec<DuplicateFile>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DuplicateFile {
    pub path: String,
    pub storage: DuplicateStorage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DuplicateStorage {
    ApfsClone,
    PhysicalCopy,
    HardLink,
    Unknown,
}

#[derive(Clone, Copy, Debug)]
struct CloneMetadata {
    clone_id: u64,
    may_share_blocks: bool,
    shares_all_blocks: bool,
}

#[derive(Debug)]
struct DuplicateCandidate {
    path: String,
    clone: Option<CloneMetadata>,
    file_identity: Option<(u64, u64)>,
}

pub(crate) fn begin_duplicate_scan() -> u64 {
    DUPLICATE_SCAN_GENERATION.fetch_add(1, Ordering::SeqCst) + 1
}

pub(crate) fn cancel_duplicate_scan() {
    DUPLICATE_SCAN_GENERATION.fetch_add(1, Ordering::SeqCst);
}

fn scan_is_current(generation: u64) -> bool {
    DUPLICATE_SCAN_GENERATION.load(Ordering::SeqCst) == generation
}

pub(crate) fn scan_duplicates(
    inputs: Vec<DuplicateInput>,
    generation: u64,
) -> DuplicateScanResponse {
    let mut by_size: HashMap<u64, Vec<DuplicateInput>> = HashMap::new();
    for input in inputs.into_iter().filter(|input| input.size > 0) {
        by_size.entry(input.size).or_default().push(input);
    }
    let unique_size_files = by_size.values().filter(|files| files.len() == 1).count();

    let mut size_groups: Vec<_> = by_size
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();
    size_groups.sort_by(|left, right| right.0.cmp(&left.0));

    let mut budget = ReadBudget::new(MAX_DUPLICATE_READ_BYTES);
    let mut scanned_files = unique_size_files;
    let mut skipped_files = 0;
    let mut limited = false;
    let mut groups = Vec::new();
    for (size, same_size) in size_groups {
        if !scan_is_current(generation) {
            return DuplicateScanResponse {
                scanned_files,
                skipped_files,
                limited,
                cancelled: true,
                groups,
            };
        }
        let mut by_partial_hash: HashMap<[u8; 32], Vec<DuplicateInput>> = HashMap::new();
        for input in same_size {
            match hash_file_partial(Path::new(&input.path), &mut budget, generation) {
                Ok(hash) => by_partial_hash.entry(hash).or_default().push(input),
                Err(HashFailure::BudgetExceeded) => {
                    limited = true;
                    skipped_files += 1;
                }
                Err(HashFailure::Cancelled) => {
                    return DuplicateScanResponse {
                        scanned_files,
                        skipped_files,
                        limited,
                        cancelled: true,
                        groups,
                    };
                }
                Err(HashFailure::Unreadable) => skipped_files += 1,
            }
        }

        let mut by_hash: HashMap<[u8; 32], Vec<DuplicateInput>> = HashMap::new();
        for same_partial in by_partial_hash.into_values() {
            if same_partial.len() == 1 {
                scanned_files += 1;
                continue;
            }
            for input in same_partial {
                match hash_file(Path::new(&input.path), &mut budget, generation) {
                    Ok(hash) => {
                        scanned_files += 1;
                        by_hash.entry(hash).or_default().push(input);
                    }
                    Err(HashFailure::BudgetExceeded) => {
                        limited = true;
                        skipped_files += 1;
                    }
                    Err(HashFailure::Cancelled) => {
                        return DuplicateScanResponse {
                            scanned_files,
                            skipped_files,
                            limited,
                            cancelled: true,
                            groups,
                        };
                    }
                    Err(HashFailure::Unreadable) => skipped_files += 1,
                }
            }
        }

        for same_content in by_hash.into_values().filter(|files| files.len() > 1) {
            let members = same_content
                .into_iter()
                .map(|input| DuplicateCandidate::from_path(input.path))
                .collect();
            groups.push(DuplicateGroup {
                size,
                files: classify_duplicate_members(members),
            });
        }
    }

    groups.sort_by(|left, right| right.size.cmp(&left.size));
    DuplicateScanResponse {
        scanned_files,
        skipped_files,
        limited,
        cancelled: false,
        groups,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HashFailure {
    BudgetExceeded,
    Cancelled,
    Unreadable,
}

struct ReadBudget {
    remaining: u64,
}

impl ReadBudget {
    fn new(bytes: u64) -> Self {
        Self { remaining: bytes }
    }

    fn consume(&mut self, bytes: usize) -> Result<(), HashFailure> {
        let bytes = bytes as u64;
        if bytes > self.remaining {
            return Err(HashFailure::BudgetExceeded);
        }
        self.remaining -= bytes;
        Ok(())
    }

    fn allowance(&self, requested: usize) -> Result<usize, HashFailure> {
        if self.remaining == 0 {
            return Err(HashFailure::BudgetExceeded);
        }
        Ok(requested.min(self.remaining as usize))
    }
}

fn hash_file_partial(
    path: &Path,
    budget: &mut ReadBudget,
    generation: u64,
) -> Result<[u8; 32], HashFailure> {
    if file_is_dataless(path) {
        return Err(HashFailure::Unreadable);
    }
    let mut file = File::open(path).map_err(|_| HashFailure::Unreadable)?;
    let length = file.metadata().map_err(|_| HashFailure::Unreadable)?.len();
    let mut hasher = Sha256::new();
    hasher.update(length.to_le_bytes());
    hash_chunk(
        &mut file,
        &mut hasher,
        budget,
        generation,
        PARTIAL_HASH_BYTES,
    )?;
    if length > PARTIAL_HASH_BYTES as u64 {
        file.seek(SeekFrom::Start(
            length.saturating_sub(PARTIAL_HASH_BYTES as u64),
        ))
        .map_err(|_| HashFailure::Unreadable)?;
        hash_chunk(
            &mut file,
            &mut hasher,
            budget,
            generation,
            PARTIAL_HASH_BYTES,
        )?;
    }
    Ok(hasher.finalize().into())
}

fn hash_chunk(
    file: &mut File,
    hasher: &mut Sha256,
    budget: &mut ReadBudget,
    generation: u64,
    limit: usize,
) -> Result<(), HashFailure> {
    if !scan_is_current(generation) {
        return Err(HashFailure::Cancelled);
    }
    let allowed = budget.allowance(limit)?;
    let mut buffer = vec![0_u8; allowed];
    let read = file
        .read(&mut buffer)
        .map_err(|_| HashFailure::Unreadable)?;
    budget.consume(read)?;
    hasher.update(&buffer[..read]);
    if allowed < limit && read == allowed {
        return Err(HashFailure::BudgetExceeded);
    }
    Ok(())
}

fn hash_file(
    path: &Path,
    budget: &mut ReadBudget,
    generation: u64,
) -> Result<[u8; 32], HashFailure> {
    if file_is_dataless(path) {
        return Err(HashFailure::Unreadable);
    }
    let mut file = File::open(path).map_err(|_| HashFailure::Unreadable)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        if !scan_is_current(generation) {
            return Err(HashFailure::Cancelled);
        }
        let allowed = budget.allowance(buffer.len())?;
        let read = file
            .read(&mut buffer[..allowed])
            .map_err(|_| HashFailure::Unreadable)?;
        if read == 0 {
            break;
        }
        budget.consume(read)?;
        hasher.update(&buffer[..read]);
        if allowed < buffer.len() && read == allowed {
            return Err(HashFailure::BudgetExceeded);
        }
    }
    Ok(hasher.finalize().into())
}

fn is_dataless_flags(flags: u32) -> bool {
    const SF_DATALESS: u32 = 0x4000_0000;
    flags & SF_DATALESS != 0
}

#[cfg(target_os = "macos")]
fn file_is_dataless(path: &Path) -> bool {
    use std::os::macos::fs::MetadataExt;
    path.metadata()
        .map(|metadata| is_dataless_flags(metadata.st_flags()))
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn file_is_dataless(_path: &Path) -> bool {
    false
}

impl DuplicateCandidate {
    fn from_path(path: String) -> Self {
        Self {
            clone: clone_metadata(Path::new(&path)),
            file_identity: file_identity(Path::new(&path)),
            path,
        }
    }

    #[cfg(test)]
    fn test(path: &str, clone_id: u64, shares_all_blocks: bool) -> Self {
        Self {
            path: path.to_owned(),
            clone: Some(CloneMetadata {
                clone_id,
                may_share_blocks: shares_all_blocks,
                shares_all_blocks,
            }),
            file_identity: None,
        }
    }

    #[cfg(test)]
    fn test_unknown(path: &str) -> Self {
        Self {
            path: path.to_owned(),
            clone: None,
            file_identity: None,
        }
    }
}

fn classify_duplicate_members(members: Vec<DuplicateCandidate>) -> Vec<DuplicateFile> {
    let mut clone_counts = HashMap::new();
    let mut identity_counts = HashMap::new();
    for member in &members {
        if let Some(clone) = member.clone.filter(|clone| clone.shares_all_blocks) {
            *clone_counts.entry(clone.clone_id).or_insert(0_usize) += 1;
        }
        if let Some(identity) = member.file_identity {
            *identity_counts.entry(identity).or_insert(0_usize) += 1;
        }
    }

    members
        .into_iter()
        .map(|member| {
            let is_hard_link = member
                .file_identity
                .and_then(|identity| identity_counts.get(&identity))
                .is_some_and(|count| *count > 1);
            let storage = if is_hard_link {
                DuplicateStorage::HardLink
            } else {
                match member.clone {
                    Some(clone)
                        if clone.shares_all_blocks
                            && clone_counts
                                .get(&clone.clone_id)
                                .is_some_and(|count| *count > 1) =>
                    {
                        DuplicateStorage::ApfsClone
                    }
                    Some(clone) if !clone.may_share_blocks => DuplicateStorage::PhysicalCopy,
                    Some(_) | None => DuplicateStorage::Unknown,
                }
            };
            DuplicateFile {
                path: member.path,
                storage,
            }
        })
        .collect()
}

#[cfg(unix)]
fn file_identity(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = path.metadata().ok()?;
    Some((metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn file_identity(_path: &Path) -> Option<(u64, u64)> {
    None
}

#[cfg(target_os = "macos")]
fn clone_metadata(path: &Path) -> Option<CloneMetadata> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt, ptr::addr_of};

    const ATTR_BIT_MAP_COUNT: u16 = 5;
    const ATTR_CMNEXT_CLONEID: u32 = 0x0000_0100;
    const ATTR_CMNEXT_EXT_FLAGS: u32 = 0x0000_0200;
    const ATTR_CMNEXT_CLONE_REFCNT: u32 = 0x0000_1000;
    const FSOPT_ATTR_CMN_EXTENDED: u32 = 0x0000_0020;
    const EF_MAY_SHARE_BLOCKS: u64 = 0x0000_0001;
    const EF_SHARES_ALL_BLOCKS: u64 = 0x0000_0040;

    #[repr(C)]
    struct AttrList {
        bitmapcount: u16,
        reserved: u16,
        commonattr: u32,
        volattr: u32,
        dirattr: u32,
        fileattr: u32,
        forkattr: u32,
    }

    #[repr(C, packed(4))]
    struct CloneAttrBuffer {
        length: u32,
        clone_id: u64,
        extended_flags: u64,
        clone_refcount: u32,
    }

    unsafe extern "C" {
        fn getattrlist(
            path: *const std::ffi::c_char,
            attributes: *mut AttrList,
            buffer: *mut std::ffi::c_void,
            buffer_size: usize,
            options: u32,
        ) -> std::ffi::c_int;
    }

    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut attributes = AttrList {
        bitmapcount: ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: 0,
        volattr: 0,
        dirattr: 0,
        fileattr: 0,
        forkattr: ATTR_CMNEXT_CLONEID | ATTR_CMNEXT_EXT_FLAGS | ATTR_CMNEXT_CLONE_REFCNT,
    };
    let mut buffer = CloneAttrBuffer {
        length: 0,
        clone_id: 0,
        extended_flags: 0,
        clone_refcount: 0,
    };
    // SAFETY: both C layouts match Darwin's attrlist/getattrlist ABI, the path is NUL-terminated,
    // and the kernel receives the exact writable buffer size.
    let result = unsafe {
        getattrlist(
            path.as_ptr(),
            &mut attributes,
            (&mut buffer as *mut CloneAttrBuffer).cast(),
            std::mem::size_of::<CloneAttrBuffer>(),
            FSOPT_ATTR_CMN_EXTENDED,
        )
    };
    if result != 0 {
        return None;
    }

    // Packed Darwin attribute buffers are only four-byte aligned; read u64 fields unaligned.
    let clone_id = unsafe { addr_of!(buffer.clone_id).read_unaligned() };
    let flags = unsafe { addr_of!(buffer.extended_flags).read_unaligned() };
    Some(CloneMetadata {
        clone_id,
        may_share_blocks: flags & EF_MAY_SHARE_BLOCKS != 0,
        shares_all_blocks: flags & EF_SHARES_ALL_BLOCKS != 0,
    })
}

#[cfg(not(target_os = "macos"))]
fn clone_metadata(_path: &Path) -> Option<CloneMetadata> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn distinguishes_full_apfs_clones_from_physical_content_copies() {
        let members = vec![
            DuplicateCandidate::test("/original", 42, true),
            DuplicateCandidate::test("/clone", 42, true),
            DuplicateCandidate::test("/full-copy", 99, false),
        ];

        let classified = classify_duplicate_members(members);

        assert_eq!(classified[0].storage, DuplicateStorage::ApfsClone);
        assert_eq!(classified[1].storage, DuplicateStorage::ApfsClone);
        assert_eq!(classified[2].storage, DuplicateStorage::PhysicalCopy);
    }

    #[test]
    fn keeps_clone_status_unknown_when_the_volume_cannot_report_clone_mapping() {
        let members = vec![
            DuplicateCandidate::test_unknown("/one"),
            DuplicateCandidate::test_unknown("/two"),
        ];

        let classified = classify_duplicate_members(members);

        assert!(
            classified
                .iter()
                .all(|item| item.storage == DuplicateStorage::Unknown)
        );
    }

    #[test]
    fn identifies_dataless_files_before_hashing() {
        assert!(is_dataless_flags(0x4000_0000));
        assert!(!is_dataless_flags(0));
    }

    #[test]
    fn partial_hash_is_only_a_prefilter_for_exact_duplicate_detection() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("cardinal-duplicates-{unique}"));
        std::fs::create_dir(&directory).expect("create test directory");
        let first = directory.join("first.bin");
        let second = directory.join("second.bin");
        let different_middle = directory.join("different-middle.bin");
        let content = vec![b'a'; PARTIAL_HASH_BYTES * 3];
        let mut different = content.clone();
        different[PARTIAL_HASH_BYTES + 1] = b'b';
        std::fs::write(&first, &content).expect("write first");
        std::fs::write(&second, &content).expect("write second");
        std::fs::write(&different_middle, &different).expect("write different");
        let inputs = [first, second, different_middle]
            .into_iter()
            .map(|path| DuplicateInput {
                path: path.to_string_lossy().into_owned(),
                size: content.len() as u64,
            })
            .collect();

        let response = scan_duplicates(inputs, begin_duplicate_scan());

        assert_eq!(response.scanned_files, 3);
        assert_eq!(response.skipped_files, 0);
        assert_eq!(response.groups.len(), 1);
        assert_eq!(response.groups[0].files.len(), 2);
        std::fs::remove_dir_all(directory).expect("remove test directory");
    }

    #[test]
    fn cancelled_scan_returns_without_hashing_candidates() {
        let generation = begin_duplicate_scan();
        cancel_duplicate_scan();
        let inputs = vec![
            DuplicateInput {
                path: "/not/read/one".to_owned(),
                size: 100,
            },
            DuplicateInput {
                path: "/not/read/two".to_owned(),
                size: 100,
            },
        ];

        let response = scan_duplicates(inputs, generation);

        assert!(response.cancelled);
        assert_eq!(response.scanned_files, 0);
    }

    #[test]
    fn full_hash_never_reads_past_the_remaining_budget() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cardinal-budget-{unique}.bin"));
        std::fs::write(&path, vec![b'a'; 4096]).expect("write budget test file");
        let mut budget = ReadBudget::new(100);

        let result = hash_file(&path, &mut budget, begin_duplicate_scan());

        assert_eq!(result, Err(HashFailure::BudgetExceeded));
        assert_eq!(budget.remaining, 0);
        std::fs::remove_file(path).expect("remove budget test file");
    }
}
