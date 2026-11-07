use super::*;

#[test]
fn channel_mixer_cmyk_identity() {
    let mut ink = vec![0.5f32, 0.25, 0.0, 0.75, 0.1, 0.2, 0.3, 0.4];
    let orig = ink.clone();
    apply_channel_mixer_ink(&ChannelMixerParams::default(), &mut ink, 2).unwrap();
    assert_eq!(ink, orig);
}

#[test]
fn channel_mixer_cmyk_blend() {
    let params = ChannelMixerParams {
        ink: [
            [0.0, 100.0, 0.0, 0.0],
            [0.0, 100.0, 0.0, 0.0],
            [0.0, 0.0, 100.0, 0.0],
            [0.0, 0.0, 0.0, 100.0],
        ],
        ..Default::default()
    };
    let mut ink = vec![0.5, 0.25, 0.0, 0.0];
    apply_channel_mixer_ink(&params, &mut ink, 1).unwrap();
    assert_eq!(ink, vec![0.25, 0.25, 0.0, 0.0]);
}

#[test]
fn channel_mixer_cmyk_constant() {
    let mut ink = vec![0.25, 0.0, 0.0, 0.0];
    let darken = ChannelMixerParams {
        ink_constant: [-50.0, 0.0, 0.0, 0.0],
        ..Default::default()
    };
    apply_channel_mixer_ink(&darken, &mut ink, 1).unwrap();
    assert_eq!(ink[0], 0.75);

    let mut ink = vec![0.25, 0.0, 0.0, 0.0];
    let lighten = ChannelMixerParams {
        ink_constant: [50.0, 0.0, 0.0, 0.0],
        ..Default::default()
    };
    apply_channel_mixer_ink(&lighten, &mut ink, 1).unwrap();
    assert_eq!(ink[0], 0.0);
}

#[test]
fn channel_mixer_cmyk_rejects_out_of_range() {
    let mut ink = vec![0.25f32; 4];
    let params = ChannelMixerParams {
        ink: [
            [201.0, 0.0, 0.0, 0.0],
            [0.0, 100.0, 0.0, 0.0],
            [0.0, 0.0, 100.0, 0.0],
            [0.0, 0.0, 0.0, 100.0],
        ],
        ..Default::default()
    };
    assert!(apply_channel_mixer_ink(&params, &mut ink, 1).is_err());
    assert_eq!(ink, vec![0.25f32; 4], "a refused mix leaves the samples");

    let short = ChannelMixerParams {
        ink_constant: [0.0, 0.0, 0.0, -201.0],
        ..Default::default()
    };
    assert!(apply_channel_mixer_ink(&short, &mut ink, 1).is_err());
    assert!(apply_channel_mixer_ink(&ChannelMixerParams::default(), &mut ink, 2).is_err());
}
