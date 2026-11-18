#pragma once

#include "batch_options.h"

#include <QtCore/QList>
#include <QtCore/QPair>
#include <QtWidgets/QDialog>
#include <QtWidgets/QWidget>

#include <functional>

class QCheckBox;
class QComboBox;
class QLabel;
class QPushButton;
class QSpinBox;

namespace pictura {

// The Batch/Create Droplet File Naming group: six editable field combos
// (tokens or custom text), the example, starting serial, and compatibility.
class FileNamingWidget : public QWidget {
    Q_OBJECT

public:
    static constexpr int kFieldCount = 6;

    explicit FileNamingWidget(QWidget* parent = nullptr);

    void setNaming(const QStringList& fields, int startingSerial, bool windows, bool mac);
    QStringList fields() const;
    int startingSerial() const;
    bool windowsCompatible() const;
    bool macCompatible() const;
    QString exampleText() const;

signals:
    void changed();

private:
    void updateExample();

    QList<QComboBox*> fields_;
    QSpinBox* serial_ = nullptr;
    QCheckBox* windows_ = nullptr;
    QCheckBox* mac_ = nullptr;
    QCheckBox* unix_ = nullptr;
    QLabel* example_ = nullptr;
};

// Every Batch option (Play, Source, Destination, File Naming, Errors), shared by
// the Batch dialog and the future Create Droplet dialog.
class BatchOptionsWidget : public QWidget {
    Q_OBJECT

public:
    using ActionLibrary = QList<QPair<QString, QStringList>>;
    using FolderProvider = std::function<QString(const QString& title, const QString& start)>;
    using FileProvider = std::function<QString(const QString& title, const QString& start)>;

    explicit BatchOptionsWidget(QWidget* parent = nullptr);

    void setLibrary(const ActionLibrary& library);
    void setOptions(const BatchOptions& options);
    BatchOptions options() const;

    void setFolderProvider(FolderProvider provider) { folderProvider_ = std::move(provider); }
    void setFileProvider(FileProvider provider) { fileProvider_ = std::move(provider); }

private:
    void fillActions();
    void updateEnablement();
    void chooseSourceFolder();
    void chooseDestinationFolder();
    void chooseErrorLog();

    ActionLibrary library_;
    QComboBox* set_ = nullptr;
    QComboBox* action_ = nullptr;
    QComboBox* source_ = nullptr;
    QPushButton* sourceChoose_ = nullptr;
    QLabel* sourcePath_ = nullptr;
    QCheckBox* overrideOpen_ = nullptr;
    QCheckBox* includeSubfolders_ = nullptr;
    QCheckBox* suppressOpen_ = nullptr;
    QCheckBox* suppressProfile_ = nullptr;
    QComboBox* destination_ = nullptr;
    QPushButton* destinationChoose_ = nullptr;
    QLabel* destinationPath_ = nullptr;
    QCheckBox* overrideSave_ = nullptr;
    FileNamingWidget* naming_ = nullptr;
    QComboBox* errors_ = nullptr;
    QPushButton* errorLogChoose_ = nullptr;
    QLabel* errorLogPath_ = nullptr;
    QString sourceFolder_;
    QString destinationFolder_;
    QString errorLog_;
    FolderProvider folderProvider_;
    FileProvider fileProvider_;
};

// File > Automate > Batch…: OK validates through the batch engine, warns about
// a baked-in Open step, and accepts only a runnable batch.
class BatchDialog : public QDialog {
    Q_OBJECT

public:
    using Validator = std::function<QString(const BatchOptions&)>;
    using OpenProbe = std::function<bool(const BatchOptions&)>;
    using MessageHandler = std::function<void(const QString& text)>;
    using ConfirmHandler = std::function<bool(const QString& text)>;

    explicit BatchDialog(QWidget* parent = nullptr);

    BatchOptionsWidget* optionsWidget() const { return options_; }
    BatchOptions options() const;

    void setValidator(Validator validator) { validator_ = std::move(validator); }
    void setOpenProbe(OpenProbe probe) { openProbe_ = std::move(probe); }
    void setMessageHandler(MessageHandler handler) { messageHandler_ = std::move(handler); }
    void setConfirmHandler(ConfirmHandler handler) { confirmHandler_ = std::move(handler); }

    void accept() override;

private:
    BatchOptionsWidget* options_ = nullptr;
    Validator validator_;
    OpenProbe openProbe_;
    MessageHandler messageHandler_;
    ConfirmHandler confirmHandler_;
};

} // namespace pictura
