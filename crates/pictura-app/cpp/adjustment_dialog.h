#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QRect>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtWidgets/QDialog>

#include <array>
#include <memory>

class QCheckBox;
class QGridLayout;
class QHBoxLayout;
class QPushButton;
class QSpinBox;
class QToolButton;
class QVBoxLayout;

namespace pictura {

class AdjustmentControls;
class ColorSwatchWidget;
class ImageView;
class PictureView;

// An Image > Adjustments dialog (Levels…, Curves…, Hue/Saturation…, and the
// rest the engine describes): the adjustment's controls, built from the same
// descriptor page as the Properties panel, a Preview checkbox that shows the
// result on the canvas while the dialog is open, and OK / Cancel. OK applies
// the adjustment to the active pixel layer within the selection as one state
// named for it; Cancel restores the layer exactly. Brightness/Contrast, Levels,
// and Curves subclass it with CS6's hand-laid-out pages (ported from
// photorust's BrightnessContrastDialog / LevelsDialog / CurvesDialog) over the
// same block, preview, and apply.
// ponytail: the generic pages have no presets menu, Auto / eyedropper buttons,
// histograms, or Load / Save; the preview covers the visible canvas section.
class AdjustmentDialog : public QDialog {
    Q_OBJECT

public:
    // Every dialog's number fields share one width, so fields line up and read
    // alike across the Adjustments dialogs (wide enough for "-0.5000" and
    // "300 %").
    static constexpr int kFieldWidth = 76;

    // `block` is the opening adjustment (`image_adjustment_default`);
    // `visible` the canvas section the preview refreshes.
    AdjustmentDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                     QWidget* parent = nullptr);
    ~AdjustmentDialog() override;

    QByteArray block() const { return block_; }
    virtual QWidget* controlForTest(const QString& key) const;

    void accept() override;
    void reject() override;

    // Preview the canvas section `visible` from now on (the view moved).
    void setPreviewArea(const QRect& visible);

protected:
    // A hand-laid-out dialog titled `title`: no generic controls. The subclass
    // builds its page, takes `previewCheck()` and `buttonColumn()`, and
    // edits through `setParam` / `setCurve`. The canvas histograms are taken
    // here, before the opening preview touches the pixels.
    AdjustmentDialog(PictureView* view, const QByteArray& block, const QRect& visible,
                     const QString& title, QWidget* parent);

    // The current block's descriptor page (`image_adjustment_page` rows).
    QStringList page() const;
    // `key`'s value in the current block, or `fallback` when the page lacks it.
    double param(const QString& key, double fallback = 0.0) const;
    // Set `key` and preview; false (the block unchanged) when the engine
    // refuses the result, e.g. Levels' input black at or past its white.
    bool setParam(const QString& key, double value);
    // Several edits in order, previewed once; none land if any is refused.
    bool setParams(const QList<QPair<QString, double>>& edits);
    // Curves `channel`'s points as `"x,y x,y …"`, and their replacement.
    QString curve(int channel) const;
    bool setCurve(int channel, const QString& points);
    // Replace the whole block (a Gradient Map's rebuilt `grdm`) and preview;
    // false, the block unchanged, when `next` is empty.
    bool setBlock(const QByteArray& next);
    // A 256-bin histogram of the canvas as the dialog opened: 0 luminosity,
    // 1 red, 2 green, 3 blue.
    const std::array<int, 256>& histogram(int channel) const;
    QCheckBox* previewCheck();
    // CS6's right-hand column: OK (default), Cancel, then one button per
    // `extra` label, reachable through `button(label)`.
    QVBoxLayout* buttonColumn(const QStringList& extra = {});
    QPushButton* button(const QString& text) const;
    // A CS6 value row at `row` / `row + 1` of `grid`: `label` (indented by
    // `indent` px) and a number field over a full-width slider, both editing
    // `key` over `min..max`. The field is named `key`, the slider
    // `key + "Slider"`.
    // With `ramp`, the slider's groove is those colours.
    QSpinBox* addStackedRow(QGridLayout* grid, int row, const QString& label,
                            const QString& key, int min, int max, int indent = 0,
                            const QList<QColor>& ramp = {}, const QString& suffix = {});
    // The newer single-column row: `label` with its number field (named
    // `key`, with `suffix`) at the right, over a full-width slider whose
    // groove is the `ramp` colours.
    QSpinBox* addRampRow(QVBoxLayout* layout, const QString& label, const QString& key, int min,
                         int max, const QList<QColor>& ramp, const QString& suffix = {});
    // A colour swatch button and its fill (also kept as its "color" property).
    ColorSwatchWidget* swatch(const QString& name);
    static void paintSwatch(ColorSwatchWidget* button, const QColor& color);
    // The newer layout's bottom row: Cancel, then OK (default), at the right.
    QHBoxLayout* bottomButtons();
    // The block key a stacked or ramp row named `key` edits; Hue/Saturation prefixes
    // the selected colour range.
    virtual QString paramKey(const QString& key) const { return key; }

private:
    void edited(const QByteArray& next);
    void preview();
    void cancelPreview();
    void takeHistograms();

    PictureView* view_ = nullptr;
    QByteArray block_;
    QRect visible_;
    QString title_;
    bool previewing_ = false;
    AdjustmentControls* controls_ = nullptr;
    QCheckBox* preview_ = nullptr;
    std::array<std::array<int, 256>, 4> histograms_{};
};

// The dialog for `kind`: a hand-laid-out page where CS6 has one, else the
// generic descriptor page.
std::unique_ptr<AdjustmentDialog> makeAdjustmentDialog(const QString& kind, PictureView* view,
                                                       const QByteArray& block,
                                                       const QRect& visible,
                                                       QWidget* parent = nullptr);

// Open the `kind` dialog (`"levels"`, …) over `view`'s active layer; true when
// it was applied. `canvas` may be panned or zoomed while the dialog is open
// (see `runDialog`); the preview follows the section it shows.
bool runAdjustmentDialog(QWidget* parent, PictureView* view, const QString& kind,
                         const QColor& foreground, const QColor& background, ImageView* canvas);

} // namespace pictura
