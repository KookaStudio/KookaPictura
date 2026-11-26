#include "merge_to_hdr_dialog.h"

#include "api/document_bridge.h"
#include "api/filters_bridge.h"
#include "dialogs.h"
#include "document_command.h"
#include "frame.h"
#include "hdr_conversion_dialog.h"
#include "load_layers_dialog.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QLocale>
#include <QtCore/QSaveFile>
#include <QtCore/QSignalBlocker>
#include <QtGui/QColor>
#include <QtGui/QDoubleValidator>
#include <QtGui/QPixmap>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QVBoxLayout>

#include <cmath>
#include <memory>
#include <optional>
#include <utility>
#include <vector>

namespace pictura {
namespace {

constexpr auto kPreviewName = "hdrMergePreview";
constexpr auto kCurveKind = "pictura-hdr-response";
constexpr auto kPresetKind = "pictura-hdr-preset";

QString tr(const char* text)
{
    return QCoreApplication::translate("MergeToHdrDialog", text);
}

rust::Slice<const double> asSlice(const std::vector<double>& numbers)
{
    return {numbers.data(), numbers.size()};
}

std::optional<std::vector<double>> exposureValues(QWidget* parent, const QStringList& paths,
                                                 const rust::Vec<double>& source)
{
    std::vector<double> values(source.begin(), source.end());
    if (hdr_bracket_base(asSlice(values)) >= 0) {
        return values;
    }
    QDialog dialog(parent);
    dialog.setObjectName(QStringLiteral("hdrMergeManualExposure"));
    dialog.setWindowTitle(tr("Set Exposure Values"));
    auto* layout = new QVBoxLayout(&dialog);
    auto* explanation = new QLabel(tr("Enter EV100 for each image. Higher EV100 means less exposure."),
                                   &dialog);
    explanation->setWordWrap(true);
    layout->addWidget(explanation);
    auto* scroll = new QScrollArea(&dialog);
    scroll->setWidgetResizable(true);
    auto* page = new QWidget(scroll);
    auto* rows = new QFormLayout(page);
    std::vector<QLineEdit*> editors;
    for (qsizetype i = 0; i < paths.size(); ++i) {
        auto* editor = new QLineEdit(page);
        editor->setObjectName(QStringLiteral("hdrMergeEv%1").arg(i));
        auto* validator = new QDoubleValidator(-64.0, 64.0, 8, editor);
        validator->setLocale(QLocale::c());
        editor->setValidator(validator);
        if (std::isfinite(values[size_t(i)])) {
            editor->setText(QString::number(values[size_t(i)], 'f', 6));
        }
        rows->addRow(QFileInfo(paths[i]).fileName(), editor);
        editors.push_back(editor);
    }
    scroll->setWidget(page);
    layout->addWidget(scroll);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    layout->addWidget(buttons);
    auto* accept = buttons->button(QDialogButtonBox::Ok);
    accept->setObjectName(QStringLiteral("hdrMergeAcceptExposure"));
    const auto refresh = [&] {
        for (size_t i = 0; i < editors.size(); ++i) {
            if (!editors[i]->hasAcceptableInput()) {
                accept->setEnabled(false);
                return;
            }
            values[i] = QLocale::c().toDouble(editors[i]->text());
        }
        accept->setEnabled(hdr_bracket_base(asSlice(values)) >= 0);
    };
    for (auto* editor : editors) {
        QObject::connect(editor, &QLineEdit::textChanged, &dialog, refresh);
    }
    QObject::connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    QObject::connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    refresh();
    return dialog.exec() == QDialog::Accepted && accept->isEnabled()
        ? std::optional(std::move(values)) : std::nullopt;
}

class MergePreview final : public QDialog {
public:
    MergePreview(QWidget* parent, const HdrBracket& bracket, const QStringList& paths,
                 const std::vector<double>& ev100, bool align);
    int depth() const { return depth_->currentData().toInt(); }
    bool install(PictureView& view);

private:
    void updatePreview();
    void rebuild();
    void exchange(bool saving, bool curve);

    const HdrBracket& bracket_;
    const QStringList& paths_;
    const std::vector<double>& ev100_;
    std::optional<rust::Box<HdrResult>> merged_;
    std::vector<double> response_;
    QLabel* image_;
    QLabel* error_;
    QPushButton* accept_;
    QPushButton* saveCurve_;
    QComboBox* depth_;
    QCheckBox* align_;
    QCheckBox* ghost_;
    QComboBox* base_;
    QDoubleSpinBox* brightness_;
};

MergePreview::MergePreview(QWidget* parent, const HdrBracket& bracket, const QStringList& paths,
                           const std::vector<double>& ev100, bool align)
    : QDialog(parent), bracket_(bracket), paths_(paths), ev100_(ev100)
{
    setObjectName(QString::fromLatin1(kPreviewName));
    setWindowTitle(tr("HDR Merge Preview"));
    auto* layout = new QVBoxLayout(this);
    image_ = new QLabel(this);
    image_->setObjectName(QStringLiteral("hdrMergeImage"));
    image_->setAlignment(Qt::AlignCenter);
    image_->setMinimumSize(360, 220);
    layout->addWidget(image_);
    error_ = new QLabel(this);
    error_->setObjectName(QStringLiteral("hdrMergeError"));
    error_->setWordWrap(true);
    layout->addWidget(error_);

    auto* form = new QFormLayout;
    brightness_ = new QDoubleSpinBox(this);
    brightness_->setObjectName(QStringLiteral("hdrMergeBrightness"));
    brightness_->setRange(-8.0, 8.0);
    brightness_->setSingleStep(0.25);
    brightness_->setSuffix(tr(" EV"));
    form->addRow(tr("Preview exposure"), brightness_);
    depth_ = new QComboBox(this);
    depth_->setObjectName(QStringLiteral("hdrMergeDepth"));
    depth_->addItem(tr("32 bits per channel"), 32);
    depth_->addItem(tr("16 bits per channel"), 16);
    depth_->addItem(tr("8 bits per channel"), 8);
    form->addRow(tr("Output depth"), depth_);
    align_ = new QCheckBox(tr("Align sources"), this);
    align_->setChecked(align);
    ghost_ = new QCheckBox(tr("Remove ghosts"), this);
    ghost_->setObjectName(QStringLiteral("hdrMergeGhost"));
    base_ = new QComboBox(this);
    base_->setObjectName(QStringLiteral("hdrMergeBase"));
    for (qsizetype i = 0; i < paths.size(); ++i) {
        base_->addItem(tr("%1 (%2 EV100)").arg(QFileInfo(paths[i]).fileName())
                           .arg(ev100[size_t(i)], 0, 'f', 2));
        base_->setItemData(int(i), paths[i], Qt::ToolTipRole);
    }
    base_->setCurrentIndex(hdr_bracket_base(asSlice(ev100)));
    form->addRow(align_);
    form->addRow(ghost_);
    form->addRow(tr("Base exposure"), base_);
    layout->addLayout(form);

    auto* actions = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    accept_ = actions->button(QDialogButtonBox::Ok);
    accept_->setObjectName(QStringLiteral("hdrMergeAccept"));
    const auto addAction = [&](const QString& label, bool saving, bool curve) {
        auto* button = actions->addButton(label, QDialogButtonBox::ActionRole);
        connect(button, &QPushButton::clicked, this, [this, saving, curve] { exchange(saving, curve); });
        return button;
    };
    addAction(tr("Load Response Curve…"), false, true)->setObjectName(QStringLiteral("hdrMergeLoadCurve"));
    saveCurve_ = addAction(tr("Save Response Curve…"), true, true);
    addAction(tr("Load Preset…"), false, false);
    addAction(tr("Save Preset…"), true, false);
    layout->addWidget(actions);
    connect(actions, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(actions, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(brightness_, &QDoubleSpinBox::valueChanged, this, &MergePreview::updatePreview);
    connect(align_, &QCheckBox::toggled, this, &MergePreview::rebuild);
    connect(ghost_, &QCheckBox::toggled, this, &MergePreview::rebuild);
    connect(base_, &QComboBox::currentIndexChanged, this, &MergePreview::rebuild);
    rebuild();
}

void MergePreview::updatePreview()
{
    image_->setPixmap(merged_ && hdr_result_error(**merged_).isEmpty()
        ? QPixmap::fromImage(hdr_result_preview(bracket_, **merged_, brightness_->value()))
        : QPixmap());
}

void MergePreview::rebuild()
{
    base_->setEnabled(ghost_->isChecked());
    const int base = ghost_->isChecked() ? base_->currentIndex() : -1;
    for (int index = 0; index < base_->count(); ++index) {
        base_->setItemData(index, index == base
            ? QVariant(QColor(Qt::darkGreen)) : QVariant(), Qt::ForegroundRole);
    }
    // Drop the previous full-size result before the replacement allocates.
    merged_.reset();
    merged_.emplace(hdr_bracket_merge(bracket_, asSlice(ev100_), align_->isChecked(), base,
                                      asSlice(response_)));
    const QString error = hdr_result_error(**merged_);
    error_->setText(error);
    accept_->setEnabled(error.isEmpty());
    saveCurve_->setEnabled(error.isEmpty());
    updatePreview();
}

bool MergePreview::install(PictureView& view)
{
    return accept_->isEnabled() && merged_
        && hdr_result_install(view, bracket_, std::move(*merged_));
}

void MergePreview::exchange(bool saving, bool curve)
{
    const QString kind = QString::fromLatin1(curve ? kCurveKind : kPresetKind);
    const QString label = curve ? tr("HDR Response Curve") : tr("HDR Merge Preset");
    const QString suffix = curve ? QStringLiteral("phdr-curve") : QStringLiteral("phdr-preset");
    const QStringList filters{tr("%1 (*.%2)").arg(label, suffix)};
    if (saving) {
        QString path = getSaveFileName(this, label, {}, filters, nullptr);
        if (!path.isEmpty() && QFileInfo(path).suffix().isEmpty()) {
            path += QLatin1Char('.') + suffix;
        }
        if (path.isEmpty()) {
            return;
        }
        for (const QString& input : paths_) {
            if (QFileInfo(path).canonicalFilePath() == QFileInfo(input).canonicalFilePath()) {
                QMessageBox::warning(this, label, tr("Choose a new file to protect the exposure."));
                return;
            }
        }
        QJsonObject data{{QStringLiteral("kind"), kind}, {QStringLiteral("version"), 1}};
        if (curve) {
            QJsonArray exponents;
            for (double exponent : hdr_result_curve(**merged_)) {
                exponents.append(exponent);
            }
            data.insert(QStringLiteral("exponents"), exponents);
        } else {
            data.insert(QStringLiteral("align"), align_->isChecked());
            data.insert(QStringLiteral("ghosts"), ghost_->isChecked());
            data.insert(QStringLiteral("depth"), depth());
            data.insert(QStringLiteral("previewEv"), brightness_->value());
        }
        QSaveFile file(path);
        const QByteArray encoded = QJsonDocument(data).toJson();
        if (!file.open(QIODevice::WriteOnly) || file.write(encoded) != encoded.size() || !file.commit()) {
            QMessageBox::warning(this, label, tr("Could not save the settings."));
        }
        return;
    }
    const QString path = getOpenFileName(this, label, {}, filters);
    if (path.isEmpty()) {
        return;
    }
    QFile file(path);
    constexpr qint64 maxSettingsBytes = 8192;
    const QByteArray bytes = file.open(QIODevice::ReadOnly) ? file.read(maxSettingsBytes + 1) : QByteArray{};
    const QJsonObject data = bytes.size() <= maxSettingsBytes
        ? QJsonDocument::fromJson(bytes).object() : QJsonObject{};
    const auto reject = [&] { QMessageBox::warning(this, label, tr("Invalid or unsupported settings file.")); };
    if (data.value(QStringLiteral("kind")).toString() != kind
        || data.value(QStringLiteral("version")).toDouble() != 1.0) {
        reject();
        return;
    }
    if (curve) {
        const auto exponents = data.value(QStringLiteral("exponents")).toArray();
        if (exponents.size() != 3) {
            reject();
            return;
        }
        std::vector<double> next;
        for (const QJsonValue value : exponents) {
            const double exponent = value.toDouble();
            if (!(exponent >= 0.5 && exponent <= 2.0)) {
                reject();
                return;
            }
            next.push_back(exponent);
        }
        response_ = std::move(next);
        rebuild();
    } else {
        const auto alignment = data.value(QStringLiteral("align"));
        const auto ghosts = data.value(QStringLiteral("ghosts"));
        const auto brightness = data.value(QStringLiteral("previewEv"));
        const int index = depth_->findData(data.value(QStringLiteral("depth")).toInt(-1));
        const double ev = brightness.toDouble();
        if (index < 0 || !alignment.isBool() || !ghosts.isBool()
            || !brightness.isDouble() || ev < -8.0 || ev > 8.0) {
            reject();
            return;
        }
        const QSignalBlocker blockAlign(align_);
        const QSignalBlocker blockGhost(ghost_);
        const QSignalBlocker blockBrightness(brightness_);
        align_->setChecked(alignment.toBool());
        ghost_->setChecked(ghosts.toBool());
        brightness_->setValue(ev);
        depth_->setCurrentIndex(index);
        rebuild();
    }
}

bool convertOutput(QWidget* parent, PictureView& view, int bits)
{
    if (bits == 32) {
        return true;
    }
    HdrConversionSpec spec;
    return HdrConversionDialog::get(parent, &spec)
        && runDocumentCommand(parent, view, tr("HDR Conversion"), [&] {
            convert_depth_method(view, bits, int(spec.method), spec.exposure_ev, spec.gamma,
                                 spec.radius, spec.strength, spec.local_gamma, spec.local_exposure,
                                 spec.detail, spec.shadow, spec.highlight, spec.vibrance,
                                 spec.saturation);
        });
}

} // namespace

void showMergeToHdrDialog(PicturaMainWindow& frame)
{
    QStringList open;
    for (int index = 0; index < frame.documentCount(); ++index) {
        const QString path = frame.documentPath(index);
        if (!path.isEmpty() && !frame.isDocumentDirty(index) && isHdrBracketFile(path)) {
            open.append(path);
        }
    }
    LoadLayersDialog picker(open, documentDirectory(frame.activeFilePath()), &frame,
                            LoadLayersDialog::Purpose::HdrBracket);
    if (picker.exec() != QDialog::Accepted) {
        return;
    }
    const QStringList paths = picker.paths();
    const auto bracket = hdr_bracket_open(paths);
    if (const QString error = hdr_bracket_error(*bracket); !error.isEmpty()) {
        QMessageBox::warning(&frame, tr("Merge To HDR Pro"), error);
        return;
    }
    const auto ev = exposureValues(&frame, paths, hdr_bracket_exposures(*bracket));
    if (!ev) {
        return;
    }
    MergePreview preview(&frame, *bracket, paths, *ev, picker.alignImages());
    if (preview.exec() != QDialog::Accepted) {
        return;
    }
    auto result = std::make_unique<PictureView>(&frame);
    if (!preview.install(*result)) {
        QMessageBox::warning(&frame, tr("Merge To HDR Pro"), tr("Could not create the HDR document."));
        return;
    }
    if (convertOutput(&frame, *result, preview.depth())) {
        frame.addDocument(result.release(), QString());
    }
}

} // namespace pictura
