use pictura_codec::preset_library::csf::{decode, encode, Intent, Policy, Space};

fn fixture() -> Vec<u8> {
    let mut profile = vec![0u8; 132];
    profile[..4].copy_from_slice(&132u32.to_be_bytes());
    profile[16..20].copy_from_slice(b"RGB ");
    profile[36..40].copy_from_slice(b"acsp");
    let mut working = b"prof\0\0\0\0".to_vec();
    working.extend(profile);
    let mut settings = vec![0; 132 + 6 * 12];
    settings[8..12].copy_from_slice(&0x0002_0000u32.to_be_bytes());
    settings[36..40].copy_from_slice(b"AsCs");
    settings[128..132].copy_from_slice(&6u32.to_be_bytes());
    let unsigned = |value: u32| {
        let mut tag = b"ui32\0\0\0\0".to_vec();
        tag.extend_from_slice(&value.to_be_bytes());
        tag
    };
    for (i, (key, value)) in [
        (*b"wRGB", working),
        (*b"pRGB", unsigned(u32::from_be_bytes(*b"conv"))),
        (*b"cInt", unsigned(1)),
        (*b"kpc ", unsigned(1)),
        (*b"dith", unsigned(0)),
        (*b"zzzz", b"opaque record".to_vec()),
    ]
    .into_iter()
    .enumerate()
    {
        let offset = settings.len() as u32;
        let at = 132 + 12 * i;
        settings[at..at + 4].copy_from_slice(&key);
        settings[at + 4..at + 8].copy_from_slice(&offset.to_be_bytes());
        settings[at + 8..at + 12].copy_from_slice(&(value.len() as u32).to_be_bytes());
        settings.extend(value);
        settings.resize(settings.len().next_multiple_of(4), 0);
    }
    let size = settings.len() as u32;
    settings[..4].copy_from_slice(&size.to_be_bytes());
    settings
}

#[test]
fn csf_round_trips_supported_settings() {
    let mut settings = decode(&fixture()).unwrap();
    assert_eq!(settings.intent(), Some(Intent::RelativeColorimetric));
    assert_eq!(settings.policy(Space::Rgb), Some(Policy::Convert));
    assert_eq!(settings.black_point_compensation(), Some(true));
    assert_eq!(settings.dither(), Some(false));
    let working = settings.profile(Space::Rgb).unwrap().to_vec();
    assert_eq!(settings.unsupported_keys(), [*b"zzzz"]);
    settings.set_intent(Intent::AbsoluteColorimetric);
    settings.set_dither(true);
    let round_trip = decode(&encode(&settings).unwrap()).unwrap();
    assert_eq!(round_trip.intent(), Some(Intent::AbsoluteColorimetric));
    assert_eq!(round_trip.dither(), Some(true));
    assert_eq!(round_trip.profile(Space::Rgb), Some(working.as_slice()));
    assert_eq!(round_trip.unsupported_keys(), [*b"zzzz"]);

    let mut bad = fixture();
    bad[136..140].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(decode(&bad).is_err());
    let mut bad = fixture();
    bad[9] = 99;
    assert!(decode(&bad).is_err());
}
