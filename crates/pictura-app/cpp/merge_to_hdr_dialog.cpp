#include "merge_to_hdr_dialog.h"

#include "api/document_bridge.h"
#include "dialogs.h"
#include "frame.h"
#include "hdr_toning_controls.h"
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
#include <QtWidgets/QHBoxLayout>

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

::rust::Slice<const double> asSlice(const std::vector<double>& numbers)
{
    return {numbers.data(), numbers.size()};
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
    std::optional<::rust::Box<HdrResult>> merged_;
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
    HdrToningControls* toning_;
    QCheckBox* smooth_;
    QComboBox* method_;
    QComboBox* zoom_;
    QDoubleSpinBox* curve_;
    std::vector<double> toneValues() const;
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
    zoom_ = new QComboBox(this);
    zoom_->setObjectName(QStringLiteral("hdrMergeZoom"));
    for (int percent : {50, 100, 200}) { zoom_->addItem(tr("%1%").arg(percent), percent); }
    zoom_->setCurrentIndex(1);
    layout->addWidget(zoom_);
    auto* thumbnails = new QHBoxLayout;
    for (qsizetype i = 0; i < paths.size(); ++i) {
        auto* thumbnail = new QLabel(this);
        thumbnail->setPixmap(QPixmap(paths[i]).scaled(96, 64, Qt::KeepAspectRatio, Qt::SmoothTransformation));
        thumbnail->setToolTip(tr("%1 (%2 EV100)").arg(QFileInfo(paths[i]).fileName()).arg(ev100[size_t(i)], 0, 'f', 2));
        auto* item = new QVBoxLayout;
        item->addWidget(thumbnail);
        item->addWidget(new QLabel(tr("%1 EV100").arg(ev100[size_t(i)], 0, 'f', 2), this));
        thumbnails->addLayout(item);
    }
    layout->addLayout(thumbnails);
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
    method_ = new QComboBox(this);
    method_->setObjectName(QStringLiteral("hdrMergeToneMethod"));
    method_->addItem(tr("Local Adaptation"));
    method_->addItem(tr("Exposure and Gamma"));
    form->addRow(tr("Tone method"), method_);
    smooth_ = new QCheckBox(tr("Edge smoothness"), this);
    smooth_->setObjectName(QStringLiteral("hdrMergeSmooth"));
    form->addRow(smooth_);
    curve_ = new QDoubleSpinBox(this);
    curve_->setObjectName(QStringLiteral("hdrMergeToneCurve"));
    curve_->setRange(-1.0, 1.0);
    curve_->setSingleStep(0.05);
    form->addRow(tr("Tone curve"), curve_);
    layout->addLayout(form);
    toning_ = new HdrToningControls(this);
    layout->addWidget(toning_);
    connect(toning_, &HdrToningControls::edited, this, &MergePreview::updatePreview);
    connect(method_, &QComboBox::currentIndexChanged, this, &MergePreview::updatePreview);
    connect(zoom_, &QComboBox::currentIndexChanged, this, &MergePreview::updatePreview);
    connect(smooth_, &QCheckBox::toggled, this, &MergePreview::updatePreview);
    connect(curve_, &QDoubleSpinBox::valueChanged, this, &MergePreview::updatePreview);

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

std::vector<double> MergePreview::toneValues() const
{
    const auto value = toning_->values();
    return {double(value.radius), value.strength, value.gamma, value.exposure,
            double(value.detail), double(value.shadow), double(value.highlight),
            double(value.vibrance), double(value.saturation),
            smooth_->isChecked() ? 1.0 : 0.0, curve_->value(), double(method_->currentIndex())};
}

void MergePreview::updatePreview()
{
    if (!merged_ || !hdr_result_error(**merged_).isEmpty()) {
        image_->clear();
        return;
    }
    const auto values = toneValues();
    const auto toned = hdr_result_tone(bracket_, **merged_, asSlice(values));
    const QImage image = hdr_result_preview(bracket_, *toned, brightness_->value());
    const int percent = zoom_->currentData().toInt();
    image_->setPixmap(QPixmap::fromImage(image.scaled(image.size() * percent / 100, Qt::KeepAspectRatio, Qt::SmoothTransformation)));
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
    if (!accept_->isEnabled() || !merged_) { return false; }
    const auto values = toneValues();
    auto toned = hdr_result_tone(bracket_, **merged_, asSlice(values));
    return hdr_result_install(view, bracket_, std::move(toned));
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
            QJsonArray tone;
            for (double value : toneValues()) { tone.append(value); }
            data.insert(QStringLiteral("tone"), tone);
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
        const auto tone = data.value(QStringLiteral("tone")).toArray();
        if (tone.size() == 12) {
            HdrToningControls::Values values;
            values.radius = tone[0].toInt();
            values.strength = tone[1].toDouble();
            values.gamma = tone[2].toDouble();
            values.exposure = tone[3].toDouble();
            values.detail = tone[4].toInt();
            values.shadow = tone[5].toInt();
            values.highlight = tone[6].toInt();
            values.vibrance = tone[7].toInt();
            values.saturation = tone[8].toInt();
            toning_->setValues(values);
            smooth_->setChecked(tone[9].toDouble() > 0.5);
            curve_->setValue(tone[10].toDouble());
            method_->setCurrentIndex(tone[11].toInt());
        }
        rebuild();
    }
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
    const auto estimated = hdr_bracket_exposures(*bracket);
    const std::vector<double> ev(estimated.begin(), estimated.end());
    auto result = std::make_unique<PictureView>(&frame);
    if (!hdr_bracket_install_stack(*result, *bracket, paths)) {
        QMessageBox::warning(&frame, tr("Merge To HDR Pro"), tr("Could not create the HDR document."));
        return;
    }
    const int previous = frame.activeDocumentIndex();
    auto* target = result.release();
    const int index = frame.addDocument(target, QString());
    MergePreview preview(&frame, *bracket, paths, ev, picker.alignImages());
    if (preview.exec() != QDialog::Accepted || !preview.install(*target)) {
        frame.closeDocument(index, false);
        if (previous >= 0) {
            frame.setActiveDocumentIndex(previous);
        }
        return;
    }
    frame.refresh();
}

} // namespace pictura
