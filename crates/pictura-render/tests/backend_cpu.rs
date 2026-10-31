use pictura_core::{BitDepth, BlendMode, Channel, ColorMode, Document, Layer, LockFlags, PsdRect};
use pictura_render::{
    composite_active, composite_gpu, composite_rgba, gpu_available, set_backend, Backend,
    GpuBackend, GpuError,
};

fn document() -> Document {
    const N: u32 = 4;
    let mut channels: Vec<Channel> = (0..4)
        .map(|id| Channel {
            id: if id == 3 { -1 } else { id as i16 },
            data: vec![128u8; (N * N) as usize].into(),
        })
        .collect();
    channels[3].data.as_mut_slice().fill(255);
    let layer = Layer {
        name: "base".into(),
        rect: PsdRect {
            top: 0,
            left: 0,
            bottom: N as i32,
            right: N as i32,
        },
        blend: BlendMode::Normal,
        opacity: 255,
        fill: 255,
        lock: LockFlags::default(),
        visible: true,
        channels,
        ..Default::default()
    };
    let mut doc = Document::new(N, N, ColorMode::Rgb, BitDepth::Eight);
    doc.layers = pictura_core::LayerStack::from_layers(vec![layer]);
    doc
}

#[test]
fn cpu_pin_composites_on_cpu() {
    set_backend(GpuBackend::Cpu).expect("the pin is fresh in this test binary");
    assert!(!gpu_available());
    let doc = document();
    assert!(matches!(composite_gpu(&doc), Err(GpuError::Unavailable)));
    let (buf, backend) = composite_active(&doc, true);
    assert_eq!(backend, Backend::Cpu);
    assert_eq!(buf.data, composite_rgba(&doc).data);
}
