#pragma once

#include <QtCore/QList>
#include <QtCore/QString>
#include <QtWidgets/QDialog>

class QCheckBox;
class QComboBox;
class QDialogButtonBox;
class QLabel;
class QLineEdit;

namespace pictura {

class PictureView;

struct SelectionDocument {
    PictureView* view = nullptr;
    QString label;
    bool active = false;
};

struct SelectionSaveRequest {
    PictureView* document = nullptr;
    bool newDocument = false;
    QString target;
    QString name;
    QString mode;
};

struct SelectionLoadRequest {
    PictureView* document = nullptr;
    QString target;
    bool invert = false;
    QString mode;
};

// Select > Save/Load Selection (SEL-012): choose a document, an extra channel
// or raster layer mask, and the combine operation. The frame performs the
// cross-document transfer and records the history state.
class SaveLoadSelectionDialog : public QDialog {
    Q_OBJECT

public:
    enum class Mode { Save, Load };

    SaveLoadSelectionDialog(Mode mode, const QList<SelectionDocument>& documents,
                            QWidget* parent = nullptr);

    SelectionSaveRequest saveRequest() const;
    SelectionLoadRequest loadRequest() const;

    int documentCountForTest() const;
    int currentDocumentForTest() const;
    int channelCountForTest() const;
    QString channelTextForTest(int index) const;
    QString operationTextForTest(int index) const;
    void setDocumentIndexForTest(int index);
    void setChannelIndexForTest(int index);
    void setOperationIndexForTest(int index);
    void setNameForTest(const QString& name);
    void setInvertForTest(bool invert);
    bool okEnabledForTest() const;

private:
    void refreshChannels();
    void refreshEnabled();
    PictureView* selectedView() const;
    bool newDocumentSelected() const;

    Mode mode_;
    QList<SelectionDocument> documents_;
    QComboBox* document_ = nullptr;
    QComboBox* channel_ = nullptr;
    QLabel* nameLabel_ = nullptr;
    QLineEdit* name_ = nullptr;
    QComboBox* operation_ = nullptr;
    QCheckBox* invert_ = nullptr;
    QDialogButtonBox* buttons_ = nullptr;
};

} // namespace pictura
