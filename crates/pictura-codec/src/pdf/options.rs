//! Save Adobe PDF settings: the value type, its compatibility/standard gating,
//! and the field-level view the dialog edits (`get` / `set` by field name, enum
//! values as their list index).

/// The Compatibility list. Acrobat 3 wrote PDF 1.2, so the list starts at
/// Acrobat 4 (design D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Compatibility {
    Acrobat4,
    Acrobat5,
    Acrobat6,
    Acrobat7,
    AcrobatX,
}

impl Compatibility {
    pub const ALL: [Compatibility; 5] = [
        Compatibility::Acrobat4,
        Compatibility::Acrobat5,
        Compatibility::Acrobat6,
        Compatibility::Acrobat7,
        Compatibility::AcrobatX,
    ];

    /// The header minor version: PDF 1.3 … 1.7.
    pub fn minor(self) -> u8 {
        3 + self as u8
    }

    pub fn label(self) -> &'static str {
        match self {
            Compatibility::Acrobat4 => "Acrobat 4 (PDF 1.3)",
            Compatibility::Acrobat5 => "Acrobat 5 (PDF 1.4)",
            Compatibility::Acrobat6 => "Acrobat 6 (PDF 1.5)",
            Compatibility::Acrobat7 => "Acrobat 7 (PDF 1.6)",
            Compatibility::AcrobatX => "Acrobat X (PDF 1.7)",
        }
    }

    /// The level for a Distiller `CompatibilityLevel`, rounding down into the
    /// list (1.2 and below → Acrobat 4).
    pub fn from_level(level: f64) -> Compatibility {
        let minor = ((level - 1.0) * 10.0 + 0.05).floor() as i64;
        Compatibility::ALL[(minor - 3).clamp(0, 4) as usize]
    }

    pub fn encryption(self) -> Encryption {
        match self {
            Compatibility::Acrobat4 => Encryption::Rc4_40,
            Compatibility::Acrobat5 | Compatibility::Acrobat6 => Encryption::Rc4_128,
            Compatibility::Acrobat7 => Encryption::Aes128,
            Compatibility::AcrobatX => Encryption::Aes256,
        }
    }
}

/// The encryption a compatibility level implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encryption {
    Rc4_40,
    Rc4_128,
    Aes128,
    Aes256,
}

impl Encryption {
    pub fn label(self) -> &'static str {
        match self {
            Encryption::Rc4_40 => "Low (40-bit RC4)",
            Encryption::Rc4_128 => "High (128-bit RC4)",
            Encryption::Aes128 => "High (128-bit AES)",
            Encryption::Aes256 => "High (256-bit AES)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standard {
    None,
    X1a2001,
    X32002,
    X42008,
    A1b2005,
}

impl Standard {
    pub const ALL: [Standard; 5] = [
        Standard::None,
        Standard::X1a2001,
        Standard::X32002,
        Standard::X42008,
        Standard::A1b2005,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Standard::None => "None",
            Standard::X1a2001 => "PDF/X-1a:2001",
            Standard::X32002 => "PDF/X-3:2002",
            Standard::X42008 => "PDF/X-4:2008",
            Standard::A1b2005 => "PDF/A-1b:2005",
        }
    }

    /// The Distiller `CheckCompliance` name.
    pub(crate) fn compliance(self) -> &'static str {
        match self {
            Standard::None => "None",
            Standard::X1a2001 => "PDFX1a:2001",
            Standard::X32002 => "PDFX3:2002",
            Standard::X42008 => "PDFX4:2008",
            Standard::A1b2005 => "PDFA1B:2005",
        }
    }

    pub fn is_pdfx(self) -> bool {
        matches!(
            self,
            Standard::X1a2001 | Standard::X32002 | Standard::X42008
        )
    }

    /// Whether the standard allows live transparency (only PDF/X-4).
    pub fn allows_transparency(self) -> bool {
        matches!(self, Standard::None | Standard::X42008)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Downsample {
    None,
    Average,
    Subsample,
    Bicubic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Zip,
    Jpeg,
    /// Listed but not encodable (design D4); `normalize` maps it to JPEG.
    Jpeg2000,
}

/// JPEG Image Quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Minimum,
    Low,
    Medium,
    High,
    Maximum,
}

impl Quality {
    pub const ALL: [Quality; 5] = [
        Quality::Minimum,
        Quality::Low,
        Quality::Medium,
        Quality::High,
        Quality::Maximum,
    ];

    /// Distiller's `ColorImageDict /QFactor` for the level.
    pub fn qfactor(self) -> f64 {
        match self {
            Quality::Minimum => 2.4,
            Quality::Low => 1.3,
            Quality::Medium => 0.76,
            Quality::High => 0.4,
            Quality::Maximum => 0.15,
        }
    }

    /// The nearest level for a QFactor.
    pub fn from_qfactor(q: f64) -> Quality {
        *Quality::ALL
            .iter()
            .min_by(|a, b| (a.qfactor() - q).abs().total_cmp(&(b.qfactor() - q).abs()))
            .expect("five levels")
    }

    /// The 0–100 encoder quality. ponytail: inferred; Distiller's QFactor is
    /// a quantizer scale with no published mapping to IJG quality.
    pub fn encoder_quality(self) -> u8 {
        match self {
            Quality::Minimum => 25,
            Quality::Low => 50,
            Quality::Medium => 70,
            Quality::High => 85,
            Quality::Maximum => 95,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorConversion {
    None,
    ToDestination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Srgb,
    AdobeRgb,
    ProPhoto,
    GrayGamma22,
    CoatedCmyk,
}

impl Destination {
    pub const ALL: [Destination; 5] = [
        Destination::Srgb,
        Destination::AdobeRgb,
        Destination::ProPhoto,
        Destination::GrayGamma22,
        Destination::CoatedCmyk,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Destination::Srgb => "sRGB IEC61966-2.1",
            Destination::AdobeRgb => "Adobe RGB (1998)",
            Destination::ProPhoto => "ProPhoto RGB",
            Destination::GrayGamma22 => "Gray Gamma 2.2",
            Destination::CoatedCmyk => "Photocraft Coated CMYK",
        }
    }

    pub fn profile(self) -> pictura_color::Profile {
        use pictura_color::Profile;
        match self {
            Destination::Srgb => Profile::srgb(),
            Destination::AdobeRgb => Profile::adobe_rgb(),
            Destination::ProPhoto => Profile::pro_photo(),
            Destination::GrayGamma22 => Profile::gray_gamma22(),
            Destination::CoatedCmyk => Profile::coated_cmyk(),
        }
    }

    /// The profile bytes embedded for this destination.
    pub(crate) fn icc(self) -> Vec<u8> {
        match self {
            Destination::CoatedCmyk => pictura_color::Profile::coated_cmyk_icc().to_vec(),
            other => other.profile().to_icc(),
        }
    }

    pub fn channels(self) -> u8 {
        match self {
            Destination::GrayGamma22 => 1,
            Destination::CoatedCmyk => 4,
            _ => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileInclusion {
    IncludeDestination,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Printing {
    None,
    LowResolution,
    HighResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Changes {
    None,
    PageAssembly,
    FormFill,
    Commenting,
    AnyExceptExtract,
}

/// The Security page. An empty open password asks for none; an empty
/// permissions password leaves every permission granted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Security {
    pub open_password: String,
    pub permissions_password: String,
    pub printing: Printing,
    pub changes: Changes,
    pub copying: bool,
    pub accessibility: bool,
    pub plaintext_metadata: bool,
}

impl Default for Security {
    fn default() -> Self {
        Security {
            open_password: String::new(),
            permissions_password: String::new(),
            printing: Printing::HighResolution,
            changes: Changes::AnyExceptExtract,
            copying: true,
            accessibility: true,
            plaintext_metadata: false,
        }
    }
}

impl Security {
    pub fn encrypts(&self) -> bool {
        !self.open_password.is_empty() || !self.permissions_password.is_empty()
    }
}

/// The PDF/X output intent fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputIntent {
    pub profile_name: String,
    pub condition: String,
    pub condition_identifier: String,
    pub registry_name: String,
}

impl Default for OutputIntent {
    fn default() -> Self {
        OutputIntent {
            profile_name: Destination::CoatedCmyk.label().into(),
            condition: "Coated offset (synthetic)".into(),
            condition_identifier: "Custom".into(),
            registry_name: "http://www.color.org".into(),
        }
    }
}

/// Everything the Save Adobe PDF dialog and the Save options decide.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfOptions {
    pub description: String,
    pub standard: Standard,
    pub compatibility: Compatibility,
    pub preserve_editing: bool,
    pub embed_thumbnails: bool,
    pub fast_web_view: bool,
    pub view_after_saving: bool,
    pub downsample: Downsample,
    pub downsample_ppi: f64,
    pub downsample_above_ppi: f64,
    pub compression: Compression,
    pub quality: Quality,
    pub tile_size: u32,
    pub convert_16_to_8: bool,
    pub color_conversion: ColorConversion,
    pub destination: Destination,
    pub profile_inclusion: ProfileInclusion,
    pub output_intent: OutputIntent,
    pub security: Security,
    pub layers: bool,
    pub notes: bool,
    pub alpha_channels: bool,
    pub spot_colors: bool,
    pub icc_profile: bool,
}

impl Default for PdfOptions {
    /// `[Photoshop Default]`.
    fn default() -> Self {
        PdfOptions {
            description: "Use these settings to create Photoshop PDF documents that keep \
                          Photoshop editing data."
                .into(),
            standard: Standard::None,
            compatibility: Compatibility::Acrobat6,
            preserve_editing: true,
            embed_thumbnails: true,
            fast_web_view: false,
            view_after_saving: false,
            downsample: Downsample::None,
            downsample_ppi: 300.0,
            downsample_above_ppi: 450.0,
            compression: Compression::Zip,
            quality: Quality::Maximum,
            tile_size: 256,
            convert_16_to_8: true,
            color_conversion: ColorConversion::None,
            destination: Destination::Srgb,
            profile_inclusion: ProfileInclusion::IncludeDestination,
            output_intent: OutputIntent::default(),
            security: Security::default(),
            layers: true,
            notes: true,
            alpha_channels: true,
            spot_colors: false,
            icc_profile: true,
        }
    }
}

/// The fields `get`/`set` address, in dialog order.
pub const FIELDS: [&str; 33] = [
    "description",
    "standard",
    "compatibility",
    "preserveEditing",
    "embedThumbnails",
    "fastWebView",
    "viewAfterSaving",
    "downsample",
    "downsamplePpi",
    "downsampleAbovePpi",
    "compression",
    "quality",
    "tileSize",
    "convert16To8",
    "colorConversion",
    "destination",
    "profileInclusion",
    "outputIntentProfile",
    "outputCondition",
    "outputConditionIdentifier",
    "registryName",
    "openPassword",
    "permissionsPassword",
    "printing",
    "changes",
    "enableCopying",
    "enableAccessibility",
    "plaintextMetadata",
    "layers",
    "notes",
    "alphaChannels",
    "spotColors",
    "iccProfile",
];

/// The `index`th variant of an enum list, or `None` when out of range.
fn pick<T: Copy>(all: &[T], value: &str) -> Option<T> {
    value
        .trim()
        .parse::<usize>()
        .ok()
        .and_then(|i| all.get(i).copied())
}

fn flag(value: &str) -> Option<bool> {
    match value.trim() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

const DOWNSAMPLES: [Downsample; 4] = [
    Downsample::None,
    Downsample::Average,
    Downsample::Subsample,
    Downsample::Bicubic,
];
const COMPRESSIONS: [Compression; 4] = [
    Compression::None,
    Compression::Zip,
    Compression::Jpeg,
    Compression::Jpeg2000,
];
const CONVERSIONS: [ColorConversion; 2] = [ColorConversion::None, ColorConversion::ToDestination];
const INCLUSIONS: [ProfileInclusion; 2] =
    [ProfileInclusion::IncludeDestination, ProfileInclusion::None];
const PRINTINGS: [Printing; 3] = [
    Printing::None,
    Printing::LowResolution,
    Printing::HighResolution,
];
const CHANGES: [Changes; 5] = [
    Changes::None,
    Changes::PageAssembly,
    Changes::FormFill,
    Changes::Commenting,
    Changes::AnyExceptExtract,
];

fn index_of<T: PartialEq>(all: &[T], value: &T) -> String {
    all.iter().position(|v| v == value).unwrap_or(0).to_string()
}

fn bit(on: bool) -> String {
    if on { "1" } else { "0" }.to_string()
}

impl PdfOptions {
    /// A field's value as text (enums as their list index, flags as `0`/`1`),
    /// plus the derived read-only `encryptionLevel`; `None` for an unknown name.
    pub fn get(&self, field: &str) -> Option<String> {
        let s = &self.security;
        let o = &self.output_intent;
        Some(match field {
            "description" => self.description.clone(),
            "standard" => index_of(&Standard::ALL, &self.standard),
            "compatibility" => index_of(&Compatibility::ALL, &self.compatibility),
            "preserveEditing" => bit(self.preserve_editing),
            "embedThumbnails" => bit(self.embed_thumbnails),
            "fastWebView" => bit(self.fast_web_view),
            "viewAfterSaving" => bit(self.view_after_saving),
            "downsample" => index_of(&DOWNSAMPLES, &self.downsample),
            "downsamplePpi" => super::object::real(self.downsample_ppi),
            "downsampleAbovePpi" => super::object::real(self.downsample_above_ppi),
            "compression" => index_of(&COMPRESSIONS, &self.compression),
            "quality" => index_of(&Quality::ALL, &self.quality),
            "tileSize" => self.tile_size.to_string(),
            "convert16To8" => bit(self.convert_16_to_8),
            "colorConversion" => index_of(&CONVERSIONS, &self.color_conversion),
            "destination" => index_of(&Destination::ALL, &self.destination),
            "profileInclusion" => index_of(&INCLUSIONS, &self.profile_inclusion),
            "outputIntentProfile" => o.profile_name.clone(),
            "outputCondition" => o.condition.clone(),
            "outputConditionIdentifier" => o.condition_identifier.clone(),
            "registryName" => o.registry_name.clone(),
            "openPassword" => s.open_password.clone(),
            "permissionsPassword" => s.permissions_password.clone(),
            "printing" => index_of(&PRINTINGS, &s.printing),
            "changes" => index_of(&CHANGES, &s.changes),
            "enableCopying" => bit(s.copying),
            "enableAccessibility" => bit(s.accessibility),
            "plaintextMetadata" => bit(s.plaintext_metadata),
            "layers" => bit(self.layers),
            "notes" => bit(self.notes),
            "alphaChannels" => bit(self.alpha_channels),
            "spotColors" => bit(self.spot_colors),
            "iccProfile" => bit(self.icc_profile),
            "encryptionLevel" => self.compatibility.encryption().label().to_string(),
            _ => return None,
        })
    }

    /// Set a field from text (see [`PdfOptions::get`]) and re-apply the
    /// gating; false, unchanged, for an unknown field or an unparsable value.
    pub fn set(&mut self, field: &str, value: &str) -> bool {
        let ppi = |v: &str| v.trim().parse::<f64>().ok().filter(|p| p.is_finite());
        let ok = match field {
            "description" => {
                self.description = value.to_string();
                true
            }
            "standard" => pick(&Standard::ALL, value)
                .map(|v| self.apply_standard(v))
                .is_some(),
            "compatibility" => pick(&Compatibility::ALL, value)
                .map(|v| self.compatibility = v)
                .is_some(),
            "preserveEditing" => flag(value).map(|v| self.preserve_editing = v).is_some(),
            "embedThumbnails" => flag(value).map(|v| self.embed_thumbnails = v).is_some(),
            "fastWebView" => flag(value).map(|v| self.fast_web_view = v).is_some(),
            "viewAfterSaving" => flag(value).map(|v| self.view_after_saving = v).is_some(),
            "downsample" => pick(&DOWNSAMPLES, value)
                .map(|v| self.downsample = v)
                .is_some(),
            "downsamplePpi" => ppi(value).map(|v| self.downsample_ppi = v).is_some(),
            "downsampleAbovePpi" => ppi(value).map(|v| self.downsample_above_ppi = v).is_some(),
            "compression" => pick(&COMPRESSIONS, value)
                .map(|v| self.compression = v)
                .is_some(),
            "quality" => pick(&Quality::ALL, value)
                .map(|v| self.quality = v)
                .is_some(),
            "tileSize" => value
                .trim()
                .parse::<u32>()
                .ok()
                .map(|v| self.tile_size = v)
                .is_some(),
            "convert16To8" => flag(value).map(|v| self.convert_16_to_8 = v).is_some(),
            "colorConversion" => pick(&CONVERSIONS, value)
                .map(|v| self.color_conversion = v)
                .is_some(),
            "destination" => pick(&Destination::ALL, value)
                .map(|v| self.destination = v)
                .is_some(),
            "profileInclusion" => pick(&INCLUSIONS, value)
                .map(|v| self.profile_inclusion = v)
                .is_some(),
            "outputIntentProfile" => {
                self.output_intent.profile_name = value.to_string();
                true
            }
            "outputCondition" => {
                self.output_intent.condition = value.to_string();
                true
            }
            "outputConditionIdentifier" => {
                self.output_intent.condition_identifier = value.to_string();
                true
            }
            "registryName" => {
                self.output_intent.registry_name = value.to_string();
                true
            }
            "openPassword" => {
                self.security.open_password = value.to_string();
                true
            }
            "permissionsPassword" => {
                self.security.permissions_password = value.to_string();
                true
            }
            "printing" => pick(&PRINTINGS, value)
                .map(|v| self.security.printing = v)
                .is_some(),
            "changes" => pick(&CHANGES, value)
                .map(|v| self.security.changes = v)
                .is_some(),
            "enableCopying" => flag(value).map(|v| self.security.copying = v).is_some(),
            "enableAccessibility" => flag(value)
                .map(|v| self.security.accessibility = v)
                .is_some(),
            "plaintextMetadata" => flag(value)
                .map(|v| self.security.plaintext_metadata = v)
                .is_some(),
            "layers" => flag(value).map(|v| self.layers = v).is_some(),
            "notes" => flag(value).map(|v| self.notes = v).is_some(),
            "alphaChannels" => flag(value).map(|v| self.alpha_channels = v).is_some(),
            "spotColors" => flag(value).map(|v| self.spot_colors = v).is_some(),
            "iccProfile" => flag(value).map(|v| self.icc_profile = v).is_some(),
            _ => false,
        };
        if ok {
            self.normalize();
        }
        ok
    }

    /// Choose a standard and the settings it starts from: PDF/X-1a converts to
    /// CMYK device colour.
    fn apply_standard(&mut self, standard: Standard) {
        self.standard = standard;
        if standard == Standard::X1a2001 {
            self.color_conversion = ColorConversion::ToDestination;
            self.destination = Destination::CoatedCmyk;
            self.profile_inclusion = ProfileInclusion::None;
        }
    }

    /// Force what the standard and compatibility level require (design D4).
    pub fn normalize(&mut self) {
        match self.standard {
            Standard::X1a2001 | Standard::X32002 => self.compatibility = Compatibility::Acrobat4,
            Standard::X42008 => {
                self.compatibility = self.compatibility.max(Compatibility::Acrobat7)
            }
            Standard::A1b2005 => self.compatibility = Compatibility::Acrobat5,
            Standard::None => {}
        }
        if self.standard == Standard::X1a2001 {
            self.color_conversion = ColorConversion::ToDestination;
            if self.destination.channels() != 4 {
                self.destination = Destination::CoatedCmyk;
            }
            self.profile_inclusion = ProfileInclusion::None;
        }
        if self.standard != Standard::None {
            self.security = Security::default();
        }
        // PDF/X-3 and X-4 measure device RGB/gray against a CMYK output intent,
        // so their continuous-tone images stay ICC-tagged.
        if matches!(self.standard, Standard::X32002 | Standard::X42008) {
            self.icc_profile = true;
            if self.destination.channels() != 4 {
                self.profile_inclusion = ProfileInclusion::IncludeDestination;
            }
        }
        if self.compression == Compression::Jpeg2000 {
            self.compression = Compression::Jpeg;
        }
        if self.compatibility < Compatibility::Acrobat6 {
            self.convert_16_to_8 = true;
        }
        if !self.convert_16_to_8 && self.compression == Compression::Jpeg {
            self.compression = Compression::Zip;
        }
        self.downsample_ppi = self.downsample_ppi.clamp(9.0, 2400.0);
        self.downsample_above_ppi = self
            .downsample_above_ppi
            .clamp(self.downsample_ppi, 2400.0 * 2.0);
        self.tile_size = self.tile_size.clamp(128, 2048);
        self.spot_colors = false;
    }

    /// Whether the standard or compatibility level forces `field`, so the
    /// dialog shows it disabled.
    pub fn locked(&self, field: &str) -> bool {
        let standard = self.standard != Standard::None;
        match field {
            "compatibility" => matches!(
                self.standard,
                Standard::X1a2001 | Standard::X32002 | Standard::A1b2005
            ),
            "convert16To8" => self.compatibility < Compatibility::Acrobat6,
            "colorConversion" | "profileInclusion" => self.standard == Standard::X1a2001,
            "outputIntentProfile"
            | "outputCondition"
            | "outputConditionIdentifier"
            | "registryName" => !self.standard.is_pdfx(),
            "openPassword"
            | "permissionsPassword"
            | "printing"
            | "changes"
            | "enableCopying"
            | "enableAccessibility" => standard,
            // `/EncryptMetadata` exists only for the V4/V5 (AES) handlers.
            "plaintextMetadata" => standard || self.compatibility < Compatibility::Acrobat7,
            "spotColors" | "tileSize" => true,
            "iccProfile" => matches!(self.standard, Standard::X32002 | Standard::X42008),
            "downsamplePpi" | "downsampleAbovePpi" => self.downsample == Downsample::None,
            "quality" => self.compression != Compression::Jpeg,
            "destination" => self.color_conversion == ColorConversion::None,
            _ => false,
        }
    }

    /// Whether the composite keeps its alpha as a soft mask.
    pub(crate) fn keeps_transparency(&self) -> bool {
        self.compatibility >= Compatibility::Acrobat5 && self.standard.allows_transparency()
    }

    /// What live text may use at these (normalized) settings.
    pub fn text_policy(&self) -> super::TextPolicy {
        super::TextPolicy {
            transparency: self.keeps_transparency(),
            cff: self.compatibility.minor() >= 6,
        }
    }

    /// The Summary page: the effective settings, then warnings.
    pub fn summary(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Standard: {}", self.standard.label()),
            format!("Compatibility: {}", self.compatibility.label()),
            format!(
                "Preserve Photoshop Editing Capabilities: {}",
                on_off(self.preserve_editing)
            ),
            format!(
                "Downsampling: {}",
                match self.downsample {
                    Downsample::None => "Do Not Downsample".to_string(),
                    kind => format!(
                        "{} to {} ppi for images above {} ppi",
                        match kind {
                            Downsample::Average => "Average Downsampling",
                            Downsample::Subsample => "Subsampling",
                            _ => "Bicubic Downsampling",
                        },
                        super::object::real(self.downsample_ppi),
                        super::object::real(self.downsample_above_ppi)
                    ),
                }
            ),
            format!(
                "Compression: {}",
                match self.compression {
                    Compression::None => "None".to_string(),
                    Compression::Zip => "ZIP".to_string(),
                    _ => format!("JPEG, {:?} quality", self.quality),
                }
            ),
            format!(
                "Convert 16 Bit/Channel Image To 8 Bit/Channel: {}",
                on_off(self.convert_16_to_8)
            ),
            format!(
                "Color Conversion: {}",
                match self.color_conversion {
                    ColorConversion::None => "No Conversion".to_string(),
                    ColorConversion::ToDestination =>
                        format!("Convert To Destination ({})", self.destination.label()),
                }
            ),
            format!(
                "Security: {}",
                if self.security.encrypts() {
                    self.compatibility.encryption().label()
                } else {
                    "None"
                }
            ),
        ];
        if !self.keeps_transparency() {
            lines.push(
                "Warning: transparency is flattened onto white at this standard or \
                 compatibility level."
                    .into(),
            );
        }
        if self.preserve_editing && self.standard != Standard::None {
            lines.push(
                "Warning: the Photoshop editing data is Pictura's own and is ignored by \
                 PDF/X and PDF/A workflows."
                    .into(),
            );
        }
        lines
    }
}

fn on_off(on: bool) -> &'static str {
    if on {
        "On"
    } else {
        "Off"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_round_trips_through_get_and_set() {
        let options = PdfOptions::default();
        for field in FIELDS {
            let value = options.get(field).expect(field);
            let mut copy = options.clone();
            assert!(copy.set(field, &value), "{field}");
            assert_eq!(copy, options, "{field}");
        }
        assert!(!PdfOptions::default().set("nope", "1"));
        assert!(!PdfOptions::default().set("compression", "9"));
    }

    #[test]
    fn standards_force_their_levels_and_clear_security() {
        let mut options = PdfOptions::default();
        options.set("openPassword", "pw");
        options.set("standard", "1");
        assert_eq!(options.compatibility, Compatibility::Acrobat4);
        assert_eq!(options.destination, Destination::CoatedCmyk);
        assert!(!options.security.encrypts());
        assert!(options.locked("compatibility") && options.locked("openPassword"));
        options.set("standard", "3");
        assert_eq!(options.compatibility, Compatibility::Acrobat7);
        options.set("compatibility", "4");
        assert_eq!(
            options.compatibility,
            Compatibility::AcrobatX,
            "X-4 allows 1.7"
        );
        options.set("standard", "4");
        assert_eq!(options.compatibility, Compatibility::Acrobat5);
    }

    #[test]
    fn older_levels_force_eight_bit_and_jpeg2000_maps_to_jpeg() {
        let mut options = PdfOptions::default();
        options.set("convert16To8", "0");
        assert!(!options.convert_16_to_8);
        options.set("compatibility", "1");
        assert!(options.convert_16_to_8 && options.locked("convert16To8"));
        options.set("compression", "3");
        assert_eq!(options.compression, Compression::Jpeg);
        assert_eq!(Compatibility::from_level(1.2), Compatibility::Acrobat4);
        assert_eq!(Compatibility::from_level(1.7), Compatibility::AcrobatX);
        assert_eq!(Quality::from_qfactor(0.5), Quality::High);
    }
}
