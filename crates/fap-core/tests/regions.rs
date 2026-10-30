use fap_core::{CoreError, Grid, Pos, RegionId, RegionSettings, Regions};
use std::collections::BTreeMap;

fn s(level: u8) -> RegionSettings {
    RegionSettings {
        level,
        margin: 2,
        name: None,
    }
}

fn ids(rows: &[&str]) -> Grid<Option<RegionId>> {
    Grid::from_rows(
        rows.iter()
            .map(|r| {
                r.split_whitespace()
                    .map(|t| {
                        if t == "." {
                            None
                        } else {
                            Some(RegionId(t.parse().unwrap()))
                        }
                    })
                    .collect()
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn new_has_no_regions() {
    let r = Regions::new(4, 3, 2);
    assert!(r.table().is_empty());
    assert!(r.map().positions().all(|p| r.region_at(p).is_none()));
    assert_eq!(r.default_settings(), s(2));
}

#[test]
fn connected_region_keeps_id() {
    let r = Regions::from_parts(
        ids(&["1 1 .", ". 1 ."]),
        BTreeMap::from([(RegionId(1), s(3))]),
        2,
    )
    .unwrap();
    assert_eq!(r.table().len(), 1);
    assert_eq!(
        r.cells(RegionId(1)),
        vec![Pos::new(0, 0), Pos::new(0, 1), Pos::new(1, 1)]
    );
}

#[test]
fn split_rule_gives_each_piece_new_id_and_same_settings() {
    let named = RegionSettings {
        level: 3,
        margin: 1,
        name: Some("x".into()),
    };
    let r = Regions::from_parts(
        ids(&["1 . 1"]),
        BTreeMap::from([(RegionId(1), named.clone())]),
        2,
    )
    .unwrap();
    assert_eq!(r.table().len(), 2);
    assert!(r.settings(RegionId(1)).is_none());
    assert!(r.table().values().all(|v| *v == named));
    let a = r.region_at(Pos::new(0, 0)).unwrap();
    let b = r.region_at(Pos::new(0, 2)).unwrap();
    assert_ne!(a, b);
    assert!(a < b, "pieces numbered in row-major order");
}

#[test]
fn split_ids_are_deterministic() {
    let r = Regions::from_parts(ids(&["1 . 1"]), BTreeMap::from([(RegionId(1), s(2))]), 2).unwrap();
    assert_eq!(r.region_at(Pos::new(0, 0)), Some(RegionId(2)));
    assert_eq!(r.region_at(Pos::new(0, 2)), Some(RegionId(3)));
}

#[test]
fn diagonal_contact_is_not_connected() {
    let r = Regions::from_parts(
        ids(&["1 .", ". 1"]),
        BTreeMap::from([(RegionId(1), s(2))]),
        2,
    )
    .unwrap();
    assert_eq!(r.table().len(), 2);
}

#[test]
fn empty_table_entries_are_dropped() {
    let r = Regions::from_parts(
        ids(&["1 ."]),
        BTreeMap::from([(RegionId(1), s(2)), (RegionId(5), s(4))]),
        2,
    )
    .unwrap();
    assert_eq!(
        r.table().keys().copied().collect::<Vec<_>>(),
        vec![RegionId(1)]
    );
}

#[test]
fn from_parts_rejects_unknown_id_and_bad_settings() {
    let e =
        Regions::from_parts(ids(&["1 7"]), BTreeMap::from([(RegionId(1), s(2))]), 2).unwrap_err();
    assert!(matches!(e, CoreError::Format(_)));
    assert!(
        e.to_string().contains("region 7") && e.to_string().contains("row 1, column 2"),
        "{e}"
    );
    assert!(Regions::from_parts(ids(&["1"]), BTreeMap::from([(RegionId(1), s(5))]), 2).is_err());
    let bad_margin = RegionSettings {
        level: 2,
        margin: 3,
        name: None,
    };
    assert!(
        Regions::from_parts(ids(&["1"]), BTreeMap::from([(RegionId(1), bad_margin)]), 2).is_err()
    );
    assert!(Regions::from_parts(ids(&["."]), BTreeMap::new(), 0).is_err());
}

#[test]
fn create_assign_normalize() {
    let mut r = Regions::new(3, 1, 2);
    let id = r.create_region(r.default_settings());
    r.assign(Pos::new(0, 0), Some(id));
    r.assign(Pos::new(0, 2), Some(id));
    r.normalize();
    assert_eq!(r.table().len(), 2);
    let next = r.create_region(s(1));
    assert!(r.table().keys().all(|k| *k <= next));
}

#[test]
fn strips_fold_leftover_into_last() {
    let r = Regions::strips(23, 2, 5, 1, 4, 2, 2);
    let widths: Vec<usize> = (1..=4).map(|i| r.cells(RegionId(i)).len() / 2).collect();
    assert_eq!(widths, vec![5, 5, 5, 8]);
    let levels: Vec<u8> = (1..=4)
        .map(|i| r.settings(RegionId(i)).unwrap().level)
        .collect();
    assert_eq!(levels, vec![1, 2, 3, 4]);
    assert!(r.map().positions().all(|p| r.region_at(p).is_some()));
    assert_eq!(r.region_at(Pos::new(1, 22)), Some(RegionId(4)));
}

#[test]
fn strips_narrower_than_strip_width_is_one_region() {
    let r = Regions::strips(3, 2, 5, 1, 4, 1, 2);
    assert_eq!(r.table().len(), 1);
    assert_eq!(
        r.settings(RegionId(1)).unwrap(),
        &RegionSettings {
            level: 1,
            margin: 1,
            name: None
        }
    );
}

#[test]
fn create_region_after_strips_continues_ids() {
    let mut r = Regions::strips(6, 2, 2, 1, 3, 1, 2);
    assert_eq!(r.create_region(s(2)), RegionId(4));
}

#[test]
fn validate_flags_bad_settings() {
    let mut r = Regions::strips(4, 1, 2, 1, 2, 1, 2);
    assert!(r.validate().is_ok());
    r.set_default_level(9);
    assert!(r.validate().is_err());
    r.set_default_level(2);
    r.settings_mut(RegionId(1)).unwrap().margin = 3;
    assert!(r.validate().is_err());
}
