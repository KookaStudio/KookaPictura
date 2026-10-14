#pragma once

// A flat key->value view of the Pictura Raw `Fltr` settings, carried across the
// Rust bridge as text. Keys are the 4-byte on-disk codes (a trailing space is
// significant); values hold the serialized text the codec uses: decimal for
// numbers, raw bytes for text and enums, comma-separated integers for curves.
// This mirrors `pictura_codec::pictura_raw_to_text`/`from_text`; a Qt test pins
// the two against the same fixture.

#include <QtCore/QByteArray>
#include <QtCore/QList>
#include <QtCore/QMap>
#include <QtCore/QString>

namespace pictura {

class AcrSettings {
public:
    QByteArray raw(const QByteArray& key, const QByteArray& fallback = {}) const;
    void setRaw(const QByteArray& key, const QByteArray& value);

    int intValue(const QByteArray& key, int fallback = 0) const;
    double doubleValue(const QByteArray& key, double fallback = 0.0) const;
    bool boolValue(const QByteArray& key, bool fallback = false) const;
    QString textValue(const QByteArray& key, const QString& fallback = {}) const;
    QList<int> curveValue(const QByteArray& key) const;

    void setInt(const QByteArray& key, int value);
    void setDouble(const QByteArray& key, double value);
    void setBool(const QByteArray& key, bool value);
    void setText(const QByteArray& key, const QString& value);
    void setCurve(const QByteArray& key, const QList<int>& value);

    QString encode() const;
    static AcrSettings decode(const QString& text);

private:
    QMap<QByteArray, QByteArray> values_;
};

// The folded keys the dialog's panels read and write. Keys ending in a space are
// written with that space, matching the on-disk codes.
namespace acr {
inline const QByteArray WBal = QByteArrayLiteral("WBal");
inline const QByteArray Temp = QByteArrayLiteral("Temp");
inline const QByteArray Tint = QByteArrayLiteral("Tint");
inline const QByteArray Exposure = QByteArrayLiteral("Ex12");
inline const QByteArray Contrast = QByteArrayLiteral("Cr12");
inline const QByteArray Highlights = QByteArrayLiteral("Hi12");
inline const QByteArray Shadows = QByteArrayLiteral("Sh12");
inline const QByteArray Whites = QByteArrayLiteral("Wh12");
inline const QByteArray Blacks = QByteArrayLiteral("Bk12");
inline const QByteArray Clarity = QByteArrayLiteral("Cl12");
inline const QByteArray Vibrance = QByteArrayLiteral("Vibr");
inline const QByteArray Saturation = QByteArrayLiteral("Strt");

inline const QByteArray GrayScale = QByteArrayLiteral("CtoG");
inline const QByteArray ProcessVersion = QByteArrayLiteral("PrVe");
inline const QByteArray CameraProfile = QByteArrayLiteral("CamP");

inline const QByteArray ParamShadows = QByteArrayLiteral("PC_S");
inline const QByteArray ParamDarks = QByteArrayLiteral("PC_D");
inline const QByteArray ParamLights = QByteArrayLiteral("PC_L");
inline const QByteArray ParamHighlights = QByteArrayLiteral("PC_H");
inline const QByteArray SplitShadow = QByteArrayLiteral("PC_1");
inline const QByteArray SplitMid = QByteArrayLiteral("PC_2");
inline const QByteArray SplitHighlight = QByteArrayLiteral("PC_3");
inline const QByteArray CurveRgb = QByteArrayLiteral("Crv ");
inline const QByteArray CurveRed = QByteArrayLiteral("CrvR");
inline const QByteArray CurveGreen = QByteArrayLiteral("CrvG");
inline const QByteArray CurveBlue = QByteArrayLiteral("CrvB");

inline const QByteArray SharpenAmount = QByteArrayLiteral("Shrp");
inline const QByteArray SharpenRadius = QByteArrayLiteral("ShpR");
inline const QByteArray SharpenDetail = QByteArrayLiteral("ShpD");
inline const QByteArray SharpenMasking = QByteArrayLiteral("ShpM");
inline const QByteArray NoiseLuminance = QByteArrayLiteral("LNR ");
inline const QByteArray NoiseLuminanceDetail = QByteArrayLiteral("LNRD");
inline const QByteArray NoiseLuminanceContrast = QByteArrayLiteral("LNRC");
inline const QByteArray NoiseColor = QByteArrayLiteral("CNR ");
inline const QByteArray NoiseColorDetail = QByteArrayLiteral("CNRD");
inline const QByteArray NoiseColorSmoothness = QByteArrayLiteral("CNRS");

inline const QByteArray ToningShadowHue = QByteArrayLiteral("STSH");
inline const QByteArray ToningShadowSat = QByteArrayLiteral("STSS");
inline const QByteArray ToningHighlightHue = QByteArrayLiteral("STHH");
inline const QByteArray ToningHighlightSat = QByteArrayLiteral("STHS");
inline const QByteArray ToningBalance = QByteArrayLiteral("STB ");

inline const QByteArray Distortion = QByteArrayLiteral("MDis");
inline const QByteArray DefringePurpleAmount = QByteArrayLiteral("DfPA");
inline const QByteArray DefringePurpleHueLo = QByteArrayLiteral("DPHL");
inline const QByteArray DefringePurpleHueHi = QByteArrayLiteral("DPPH");
inline const QByteArray DefringeGreenAmount = QByteArrayLiteral("DfGA");
inline const QByteArray DefringeGreenHueLo = QByteArrayLiteral("DPGL");
inline const QByteArray DefringeGreenHueHi = QByteArrayLiteral("DPGH");
inline const QByteArray LensVignetteAmount = QByteArrayLiteral("VigA");
inline const QByteArray LensVignetteMidpoint = QByteArrayLiteral("VigM");

inline const QByteArray Dehaze = QByteArrayLiteral("Dhze");
inline const QByteArray GrainAmount = QByteArrayLiteral("GRNA");
inline const QByteArray GrainSize = QByteArrayLiteral("GRNS");
inline const QByteArray GrainFrequency = QByteArrayLiteral("GRNF");
inline const QByteArray PcvAmount = QByteArrayLiteral("PCVA");
inline const QByteArray PcvMidpoint = QByteArrayLiteral("PCVM");
inline const QByteArray PcvFeather = QByteArrayLiteral("PCVF");
inline const QByteArray PcvRoundness = QByteArrayLiteral("PCVR");
inline const QByteArray PcvStyle = QByteArrayLiteral("PCVS");
inline const QByteArray PcvHighlights = QByteArrayLiteral("PCVH");

inline const QByteArray RedHue = QByteArrayLiteral("RHue");
inline const QByteArray RedSat = QByteArrayLiteral("RSat");
inline const QByteArray GreenHue = QByteArrayLiteral("GHue");
inline const QByteArray GreenSat = QByteArrayLiteral("GSat");
inline const QByteArray BlueHue = QByteArrayLiteral("BHue");
inline const QByteArray BlueSat = QByteArrayLiteral("BSat");
inline const QByteArray ShadowTint = QByteArrayLiteral("BlkB");

// The eight HSL/grayscale range codes, in on-disk order.
inline const char kRangeCodes[8] = {'R', 'O', 'Y', 'G', 'A', 'B', 'P', 'M'};
inline const char* const kRangeNames[8] = {"Reds", "Oranges", "Yellows", "Greens",
                                           "Aquas", "Blues",   "Purples", "Magentas"};
} // namespace acr

} // namespace pictura
