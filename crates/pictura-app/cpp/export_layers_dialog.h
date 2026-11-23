#pragma once

#include "export_batch.h"

#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QLineEdit;

namespace pictura {

class PictureView;

// File > Scripts > Export Layers to Files: destination, prefix, visible-only,
// file type, and the ICC option.
class ExportLayersDialog : public QDialog {
    Q_OBJECT

public:
    ExportLayersDialog(const QString& defaultFolder, const QString& defaultPrefix,
                       QWidget* parent = nullptr);

    QString destination() const;
    QString prefix() const;
    bool visibleOnly() const;
    QString format() const;
    bool includeIcc() const;

private:
    void browse();
    void refreshIcc();

    QLineEdit* folder_ = nullptr;
    QLineEdit* prefix_ = nullptr;
    QCheckBox* visible_ = nullptr;
    QComboBox* format_ = nullptr;
    QCheckBox* icc_ = nullptr;
};

// The `.psd`, `.jpg`, `.tif`, `.bmp`, or `.pdf` suffix for a File Type.
QString exportLayersSuffix(const QString& format);

// The batch spec the export runs, for tests that replace only the writer.
ExportBatchSpec layersExportBatchSpec(PictureView& view, QWidget* parent,
                                      const QString& folder, const QString& prefix,
                                      bool visibleOnly, const QString& format,
                                      bool includeIcc);

// Run the dialog and export the layers. False without a document, on a bad
// destination, or when a file fails.
bool exportLayersFromView(QWidget* parent, PictureView* view);

// The dialog's work without the dialog, for the Qt test suite.
bool exportLayers(PictureView* view, const QString& folder, const QString& prefix,
                  bool visibleOnly, const QString& format, bool includeIcc,
                  QWidget* parent = nullptr);

} // namespace pictura
