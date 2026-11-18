use super::*;

#[test]
fn coverage_sources_preserve_selection_and_channel_bytes() {
    for tiled in [false, true] {
        let mut data: Plane<u8> = vec![0, 1, 64, 128, 254, 255].into();
        if tiled {
            assert!(data.tile(3, 2));
        }
        let selection = Selection {
            width: 3,
            height: 2,
            data,
        };
        let before = selection.clone();
        let channel = selection.to_channel(1);
        let (a, b) = (selection.coverage(), channel.coverage(3, 2).unwrap());
        pictura_core::assert_no_materialization(|| {
            assert_eq!(a.row(0, 0, 3), vec![0, 1, 64]);
            assert_eq!(b.row(0, 1, 3), vec![128, 254, 255]);
            assert_eq!(a.sample(-1, 0), 0);
            assert_eq!(b.sample(3, 0), 0);
        });
        assert_eq!(selection, before);
        assert!(channel.coverage(2, 2).is_none());
        assert!(Selection::from_channel(&channel, 2, 2).is_err());
    }
}
