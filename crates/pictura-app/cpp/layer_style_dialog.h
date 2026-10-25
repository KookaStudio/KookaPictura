#pragma once

// CS6's Layer Style dialog: the effect list down the left (Blending Options,
// then the ten effects in the order they apply, each with its checkbox), one
// settings page per row, and OK / Cancel. Every control writes straight into
// the layer's style as it moves, so the canvas is the preview; OK records the
// visit as one history state and Cancel (or Escape, or the close box) drops
// every edit. Ported from photorust's LayerStyleDialog.
// ponytail: no contour, texture, gradient editor, Blend If, channel
// restrictions, New Style or Make / Reset to Default; the gradient is two
// colour stops, the stroke a colour fill, and the Pattern Overlay picks from
// the eight built-in patterns.

#include <QtCore/QPointF>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QListWidget;
class QPushButton;
class QStackedWidget;

namespace pictura {

class ColorSwatchWidget;
class PictureView;

class LayerStyleDialog : public QDialog {
    Q_OBJECT

public:
    // `effect` is the page to open on — "dropShadow", "stroke" and so on, or
    // empty for Blending Options. Opening on an effect switches it on, which
    // is what picking its menu entry asks for.
    LayerStyleDialog(PictureView* view, const QString& path, const QString& effect,
                     QWidget* parent = nullptr);

    // The dialog keys of the effect rows, row order; Blending Options is "".
    QStringList rowKeys() const { return keys_; }
    QListWidget* effectList() const { return list_; }
    int currentPage() const;

    // The Distance slider's 0..1000 position for `px` and back: exponential,
    // 0 to 30000 px with about 61 px at the midpoint, as CS6's.
    static int distanceToSlider(double px);
    static double sliderToDistance(int position);

    // Place the current page's shadow at image-space `offset` from the
    // content, as a canvas drag does: Angle and Distance follow. False when
    // the current page has no draggable shadow.
    bool setShadowOffset(const QPointF& offset);

    void accept() override;
    // Overridden rather than wired to Cancel: Escape and the close box reject
    // the dialog directly, and each must drop the live edits too.
    void reject() override;

protected:
    void showEvent(QShowEvent* event) override;
    void hideEvent(QHideEvent* event) override;
    // A left drag on the canvas behind the dialog moves the current shadow.
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void addFixedPage(const QString& title, QWidget* page);
    void addEffect(const QString& key, const QString& title, QWidget* page);
    void onListChanged();

    QWidget* buildBlendingOptionsPage();
    QWidget* buildBevelPage();
    QWidget* buildStrokePage();
    QWidget* buildShadowPage(const QString& key, bool inner);
    QWidget* buildGlowPage(const QString& key, bool inner);
    QWidget* buildSatinPage();
    QWidget* buildColorOverlayPage();
    QWidget* buildGradientOverlayPage();
    QWidget* buildPatternOverlayPage();

    // Controls bind by key: each reads its start value and writes back to the
    // same key as it changes.
    void bindCheck(QCheckBox* box, const QString& key);
    void bindChoice(QComboBox* combo, const QString& key);
    void bindBlendMode(QComboBox* combo, const QString& key);
    void bindColor(ColorSwatchWidget* button, const QString& key);
    QWidget* colorButton(const QString& key);
    // CS6's slider-and-number pair on one setting.
    QWidget* sliderRow(const QString& key, int min, int max, const QString& suffix);
    // The same, on CS6's exponential 0–30000 px Distance scale.
    QWidget* distanceRow(const QString& key);
    // The current page's shadow key ("dropShadow" / "innerShadow"), or empty.
    QString shadowKey() const;
    // Show `key`'s current value in its controls without writing it back.
    void refreshControls(const QString& key);
    // CS6's angle dial and the number beside it.
    QWidget* angleRow(const QString& key);

    double value(const QString& key) const;
    void setValue(const QString& key, double value);

    PictureView* view_ = nullptr;
    QString path_;
    QStringList keys_;
    QListWidget* list_ = nullptr;
    // Groups expose Blending Options only until group effects render.
    bool blendingOnly_ = false;
    QStackedWidget* pages_ = nullptr;
    bool dirty_ = false;
    // A canvas drag in progress: the press point and the shadow's offset then.
    bool dragging_ = false;
    QPointF dragStart_;
    QPointF dragOffset_;
};

} // namespace pictura
