#pragma once

#include <QtCore/QSizeF>
#include <QtWidgets/QDialog>

class QCheckBox;

namespace pictura {

class RasterSizeFields;

// File > Open on an EPS (docs/10-workflow-io/open-and-new.md, *Open EPS*):
// the Rasterize Generic EPS Format dialog. Width and Height start at the
// bounding box, Resolution at 300 Pixels/Inch (inferred, matching Import PDF).
class RasterizeEpsDialog : public QDialog {
    Q_OBJECT

public:
    explicit RasterizeEpsDialog(QSizeF boundingPoints, QWidget* parent = nullptr);

    RasterSizeFields* sizeFields() const { return size_; }
    bool antiAliased() const;

    // Test seams: "antiAliased", else the Image Size fields' names.
    void setFieldForTest(const QString& name, const QString& value);
    QString fieldForTest(const QString& name) const;

private:
    RasterSizeFields* size_ = nullptr;
    QCheckBox* antiAliased_ = nullptr;
};

} // namespace pictura
