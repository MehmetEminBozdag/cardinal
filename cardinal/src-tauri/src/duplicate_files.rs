#[cfg(test)]
mod tests {
    use super::*;

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

        assert!(classified.iter().all(|item| item.storage == DuplicateStorage::Unknown));
    }
}
