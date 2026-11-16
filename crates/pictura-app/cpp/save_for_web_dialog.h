#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QVector>
#include <QtGui/QImage>
#include <QtWidgets/QDialog>

#include "save_for_web_settings.h"

class QCheckBox;
class QComboBox;
class QLabel;
class QPushButton;
class QSpinBox;
class QStackedWidget;
class QTabBar;

namespace pictura {

class ColorTableWidget;
class PictureView;

// File > Save for Web & Devices: CS6's web-optimisation dialog. Original /
// Optimized / 2-Up / 4-Up previews, per-pane settings, the colour-table editor
// for GIF/PNG-8, a dialog-level Metadata menu, and Image Size. Save writes the
// active pane's optimised bytes; for a sliced document it opens Save Optimized
// As and writes the slice images plus HTML/CSS. Done keeps the active pane's
// settings and the Metadata choice for the next opening.
class SaveForWebDialog : public QDialog {
    Q_OBJECT

public:
    enum Format { Gif, Jpeg, Png8, Png24, Wbmp };

    explicit SaveForWebDialog(const QImage& image, QWidget* parent = nullptr);
    SaveForWebDialog(const QImage& image, WebMetadata metadata,
                     QWidget* parent = nullptr);

    // The active pane's optimised file for the current settings, and its suffix.
    QByteArray encoded() const;
    QString suffix() const;
    // The image Image Size makes, before optimisation.
    QImage sized() const;
    // Write `encoded()` to `path`; false on a write failure.
    bool saveTo(const QString& path) const;
    // Write the slice images and/or HTML for the active pane's settings.
    bool writeSlices(const QString& path, int format, int layout,
                     const QVector<SlicePlacement>& cells, const QString& title) const;

    // Build scaled, named export cells from the document's resolved slices.
    static QVector<SlicePlacement> buildSliceCells(const QVector<ExportSlice>& slices,
                                                   const QSize& documentSize,
                                                   const QSize& targetSize,
                                                   const QString& base,
                                                   const QString& suffix);

    // Test and flow hooks.
    int activePaneForTest() const { return activePane_; }
    WebSettings paneSettingsForTest(int pane) const;
    void setActivePaneForTest(int pane) { setActivePane(pane); }
    void repopulateForTest() { repopulate(); }
    QVector<QRgb> colorTableForTest() const;
    void editSwatchForTest(int index, const QColor& color);
    void deleteSwatchForTest(int index);
    void mapSwatchForTest(int index, bool transparent);
    void sortSwatchesForTest(int sort);
    void addSwatchForTest(const QColor& color);
    bool saveColorTableForTest(const QString& path) const;
    bool loadColorTableForTest(const QString& path);

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    void buildUi();
    void applyPreset(int index);
    void refresh();
    void remember() const;
    void restore();
    void storeActive();
    void applySettingsToWidgets(const WebSettings& settings);
    void setActivePane(int pane);
    void repopulate();
    QByteArray encodedForPane(int pane) const;
    void syncColorTable();
    void onColorsEdited(const QVector<QRgb>& colors);
    void onAddColor(QRgb color);
    void onDeleteColor(int index);
    void onSortColors(int sort);
    void onTransparency(int index, bool transparent);
    void onLoadColorTable();
    void onSaveColorTable();
    void storeMetadataLevel();
    QString formatName(int format) const;

    QImage original_;
    WebMetadata rawMetadata_;
    WebSettings panes_[4];
    int activePane_ = 1;
    int metadataLevel_ = 0;

    QTabBar* tabs_ = nullptr;
    QVector<QLabel*> views_;
    QVector<QLabel*> infos_;
    QComboBox* preset_ = nullptr;
    QComboBox* format_ = nullptr;
    QStackedWidget* settings_ = nullptr;
    // GIF / PNG-8
    QComboBox* reduction_ = nullptr;
    QSpinBox* colors_ = nullptr;
    QComboBox* dither_ = nullptr;
    QSpinBox* ditherAmount_ = nullptr;
    QCheckBox* transparency_ = nullptr;
    QCheckBox* interlaced_ = nullptr;
    QComboBox* matte_ = nullptr;
    // JPEG
    QComboBox* qualityPreset_ = nullptr;
    QSpinBox* quality_ = nullptr;
    QCheckBox* progressive_ = nullptr;
    QCheckBox* optimized_ = nullptr;
    // PNG-24
    QCheckBox* pngTransparency_ = nullptr;
    // WBMP
    QComboBox* wbmpDither_ = nullptr;
    QSpinBox* wbmpAmount_ = nullptr;
    // Image Size
    QSpinBox* width_ = nullptr;
    QSpinBox* height_ = nullptr;
    QSpinBox* percent_ = nullptr;
    QCheckBox* constrain_ = nullptr;
    QComboBox* resample_ = nullptr;
    // Metadata and colour table
    QComboBox* metadata_ = nullptr;
    ColorTableWidget* colorTable_ = nullptr;
    QPushButton* repopulate_ = nullptr;
};

// Run the dialog on `view`'s flattened image and, on Save, write the optimised
// file, or the slice export when the document has user slices. False when
// cancelled, without a document, or on a write failure.
bool saveForWebFromView(QWidget* parent, PictureView* view, const QString& documentName);

} // namespace pictura
