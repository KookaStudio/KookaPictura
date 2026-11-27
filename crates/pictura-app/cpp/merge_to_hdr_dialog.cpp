#include "merge_to_hdr_dialog.h"

#include "api/document_bridge.h"
#include "dialogs.h"
#include "frame.h"
#include "hdr_toning_controls.h"
#include "load_layers_dialog.h"
#include "panels/curve_widget.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QEvent>
#include <QtCore/QTimer>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSaveFile>
#include <QtCore/QSignalBlocker>
#include <QtGui/QColor>
#include <QtGui/QPixmap>
#include <QtGui/QStandardItemModel>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMessageBox>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QTabWidget>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <array>
#include <cstdint>
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
                 const std::vector<double>& ev100, bool align, PictureView& target);
    int depth() const { return depth_->currentData().toInt(); }
    bool install(PictureView& view);

protected:
    bool eventFilter(QObject* object, QEvent* event) override;

private:
    void updatePreview();
    void displayPreview();
    void showSelectedPoint();
    QWidget* makeCurvePage();
    QWidget* makeSourcesStrip();
    void updateSourceInclusion();
    std::vector<double> toneLut() const;
    void rebuild();
    void exchange(bool saving, bool curve);

    const HdrBracket& bracket_;
    const QStringList& paths_;
    const std::vector<double>& ev100_;
    PictureView& target_;
    std::vector<QCheckBox*> sources_;
    std::vector<uint32_t> included_;
    int preferredBase_ = -1;
    QImage previewImage_;
    QScrollArea* previewScroll_;
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
    CurveWidget* curve_;
    QDoubleSpinBox* curveInput_;
    QDoubleSpinBox* curveOutput_;
    QCheckBox* corner_;
    double legacyCurve_ = 0.0;
    std::vector<double> toneValues() const;
};

MergePreview::MergePreview(QWidget* parent, const HdrBracket& bracket, const QStringList& paths,
                           const std::vector<double>& ev100, bool align, PictureView& target)
    : QDialog(parent), bracket_(bracket), paths_(paths), ev100_(ev100), target_(target)
{
    setObjectName(QString::fromLatin1(kPreviewName));
    setWindowTitle(tr("HDR Merge Preview"));
    setWindowState(windowState() | Qt::WindowFullScreen);
    auto* layout = new QVBoxLayout(this);
    auto* body = new QHBoxLayout;
    layout->addLayout(body, 1);
    auto* left = new QVBoxLayout;
    body->addLayout(left, 1);

    previewScroll_ = new QScrollArea(this);
    previewScroll_->setObjectName(QStringLiteral("hdrMergePreviewScroll"));
    previewScroll_->setAlignment(Qt::AlignCenter);
    previewScroll_->setWidgetResizable(true);
    previewScroll_->viewport()->installEventFilter(this);
    image_ = new QLabel;
    image_->setObjectName(QStringLiteral("hdrMergeImage"));
    image_->setAlignment(Qt::AlignCenter);
    image_->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Ignored);
    previewScroll_->setWidget(image_);
    left->addWidget(previewScroll_, 1);
    zoom_ = new QComboBox(this);
    zoom_->setObjectName(QStringLiteral("hdrMergeZoom"));
    zoom_->addItem(tr("Fit"), 0);
    for (int percent : {50, 100, 200}) {
        zoom_->addItem(tr("%1%").arg(percent), percent);
    }
    left->addWidget(zoom_, 0, Qt::AlignLeft);
    error_ = new QLabel(this);
    error_->setObjectName(QStringLiteral("hdrMergeError"));
    error_->setWordWrap(true);
    left->addWidget(error_);

    auto* right = new QWidget(this);
    right->setMinimumWidth(250);
    right->setMaximumWidth(430);
    body->addWidget(right);
    auto* rightLayout = new QVBoxLayout(right);
    rightLayout->setContentsMargins(0, 0, 0, 0);
    auto* controlScroll = new QScrollArea(right);
    controlScroll->setWidgetResizable(true);
    rightLayout->addWidget(controlScroll, 1);
    auto* controls = new QWidget;
    auto* settings = new QVBoxLayout(controls);
    controlScroll->setWidget(controls);
    layout->addWidget(makeSourcesStrip());

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
    preferredBase_ = hdr_bracket_base(asSlice(ev100));
    base_->setCurrentIndex(preferredBase_);
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
    settings->addLayout(form);
    auto* tabs = new QTabWidget(controls);
    tabs->setObjectName(QStringLiteral("hdrMergeTabs"));
    auto* advanced = new QWidget;
    toning_ = new HdrToningControls(controls, advanced);
    settings->addWidget(toning_);
    tabs->addTab(advanced, tr("Advanced"));
    tabs->addTab(makeCurvePage(), tr("Curve"));
    settings->addWidget(tabs);
    settings->addStretch();
    connect(toning_, &HdrToningControls::edited, this, &MergePreview::updatePreview);
    connect(method_, &QComboBox::currentIndexChanged, this, &MergePreview::updatePreview);
    connect(zoom_, &QComboBox::currentIndexChanged, this, &MergePreview::displayPreview);
    connect(smooth_, &QCheckBox::toggled, this, &MergePreview::updatePreview);

    auto* actions = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, right);
    accept_ = actions->button(QDialogButtonBox::Ok);
    accept_->setObjectName(QStringLiteral("hdrMergeAccept"));
    auto* presets = new QComboBox(controls);
    presets->setObjectName(QStringLiteral("hdrMergePreset"));
    presets->addItem(tr("Preset"));
    presets->addItem(tr("Load Preset…"));
    presets->addItem(tr("Save Preset…"));
    form->insertRow(0, presets);
    connect(presets, &QComboBox::activated, this, [this, presets](int index) {
        presets->setCurrentIndex(0);
        if (index > 0) {
            exchange(index == 2, false);
        }
    });
    auto* response = new QHBoxLayout;
    auto* loadCurve = new QPushButton(tr("Load Response…"), controls);
    loadCurve->setObjectName(QStringLiteral("hdrMergeLoadCurve"));
    saveCurve_ = new QPushButton(tr("Save Response…"), controls);
    response->addWidget(loadCurve);
    response->addWidget(saveCurve_);
    settings->insertLayout(settings->count() - 1, response);
    connect(loadCurve, &QPushButton::clicked, this, [this] { exchange(false, true); });
    connect(saveCurve_, &QPushButton::clicked, this, [this] { exchange(true, true); });
    rightLayout->addWidget(actions);
    connect(actions, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(actions, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(brightness_, &QDoubleSpinBox::valueChanged, this, &MergePreview::updatePreview);
    connect(align_, &QCheckBox::toggled, this, &MergePreview::rebuild);
    connect(ghost_, &QCheckBox::toggled, this, &MergePreview::rebuild);
    connect(base_, &QComboBox::currentIndexChanged, this, [this](int index) {
        preferredBase_ = index;
        rebuild();
    });
    updateSourceInclusion();
}

QWidget* MergePreview::makeSourcesStrip()
{
    auto* strip = new QScrollArea(this);
    strip->setObjectName(QStringLiteral("hdrMergeSources"));
    strip->setWidgetResizable(true);
    strip->setVerticalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    strip->setHorizontalScrollBarPolicy(Qt::ScrollBarAsNeeded);
    strip->setFixedHeight(126);
    auto* content = new QWidget;
    auto* thumbnails = new QHBoxLayout(content);
    for (qsizetype i = 0; i < paths_.size(); ++i) {
        auto* item = new QVBoxLayout;
        auto* thumbnail = new QLabel;
        thumbnail->setAlignment(Qt::AlignCenter);
        thumbnail->setFixedSize(116, 66);
        const QImage source = hdr_bracket_thumbnail(bracket_, uint32_t(i));
        thumbnail->setPixmap(QPixmap::fromImage(source.scaled(thumbnail->size(), Qt::KeepAspectRatio,
                                                              Qt::SmoothTransformation)));
        auto* include = new QCheckBox(tr("%1 EV100").arg(ev100_[size_t(i)], 0, 'f', 2));
        const QString name = QFileInfo(paths_[i]).fileName();
        include->setObjectName(QStringLiteral("hdrMergeSource%1").arg(i));
        include->setAccessibleName(tr("Include %1").arg(name));
        include->setToolTip(name);
        thumbnail->setToolTip(name);
        include->setChecked(true);
        sources_.push_back(include);
        item->addWidget(thumbnail);
        item->addWidget(include);
        thumbnails->addLayout(item);
        connect(include, &QCheckBox::toggled, this, [this, i](bool enabled) {
            target_.set_layer_visible(int(i) + 1, enabled);
            updateSourceInclusion();
        });
    }
    thumbnails->addStretch();
    strip->setWidget(content);
    return strip;
}

void MergePreview::updateSourceInclusion()
{
    included_.clear();
    for (size_t i = 0; i < sources_.size(); ++i) {
        if (sources_[i]->isChecked()) {
            included_.push_back(uint32_t(i));
        }
    }
    for (auto* source : sources_) {
        source->setEnabled(!source->isChecked() || included_.size() > 2);
    }
    rebuild();
}

QWidget* MergePreview::makeCurvePage()
{
    auto* page = new QWidget;
    auto* layout = new QVBoxLayout(page);
    curve_ = new CurveWidget(page);
    curve_->setObjectName(QStringLiteral("hdrMergeToneCurve"));
    curve_->setShowRamps(true);
    curve_->setToolTip(tr("Input luminance histogram. Highlights above white share the last bin."));
    layout->addWidget(curve_);
    auto* fields = new QFormLayout;
    auto* buttons = new QHBoxLayout;
    const auto makePercent = [page, fields](const QString& label, const QString& name) {
        auto* field = new QDoubleSpinBox(page);
        field->setObjectName(name);
        field->setRange(0.0, 100.0);
        field->setSingleStep(100.0 / 255.0);
        field->setSuffix(QStringLiteral(" %"));
        fields->addRow(label, field);
        return field;
    };
    curveInput_ = makePercent(tr("Input"), QStringLiteral("hdrMergeCurveInput"));
    curveOutput_ = makePercent(tr("Output"), QStringLiteral("hdrMergeCurveOutput"));
    layout->addLayout(fields);
    corner_ = new QCheckBox(tr("Corner"), page);
    corner_->setObjectName(QStringLiteral("hdrMergeCurveCorner"));
    corner_->setToolTip(tr("Connect points with straight segments"));
    auto* reset = new QPushButton(tr("Reset"), page);
    reset->setObjectName(QStringLiteral("hdrMergeCurveReset"));
    buttons->addWidget(corner_);
    buttons->addStretch();
    buttons->addWidget(reset);
    layout->addLayout(buttons);

    const auto move = [this] {
        curve_->moveSelected(qRound(curveInput_->value() * 2.55),
                             qRound(curveOutput_->value() * 2.55));
    };
    connect(curveInput_, &QDoubleSpinBox::editingFinished, this, move);
    connect(curveOutput_, &QDoubleSpinBox::editingFinished, this, move);
    connect(curve_, &CurveWidget::selectionChanged, this, &MergePreview::showSelectedPoint);
    connect(curve_, &CurveWidget::curveChanged, this, [this] {
        showSelectedPoint();
        updatePreview();
    });
    connect(corner_, &QCheckBox::toggled, this, [this](bool corner) {
        curve_->setLinearInterpolation(corner);
        updatePreview();
    });
    connect(reset, &QPushButton::clicked, this, [this] {
        legacyCurve_ = 0.0;
        const QSignalBlocker block(corner_);
        corner_->setChecked(false);
        curve_->setLinearInterpolation(false);
        curve_->resetCurve();
        showSelectedPoint();
        updatePreview();
    });
    showSelectedPoint();
    return page;
}

void MergePreview::showSelectedPoint()
{
    const int index = curve_->selected();
    const auto points = curve_->points();
    curveInput_->setEnabled(index > 0 && index + 1 < points.size());
    curveOutput_->setEnabled(index >= 0);
    const QPointF point = index >= 0 ? points.at(index) : QPointF{};
    const QSignalBlocker blockInput(curveInput_);
    const QSignalBlocker blockOutput(curveOutput_);
    curveInput_->setValue(point.x() * 100.0);
    curveOutput_->setValue(point.y() * 100.0);
}

std::vector<double> MergePreview::toneLut() const
{
    const auto points = curve_->points();
    if (std::all_of(points.begin(), points.end(), [](QPointF p) { return p.x() == p.y(); })) {
        return {};
    }
    const auto& samples = curve_->normalizedSamples();
    return std::vector<double>(samples, samples + 256);
}

std::vector<double> MergePreview::toneValues() const
{
    const auto value = toning_->values();
    return {double(value.radius), value.strength, value.gamma, value.exposure,
            double(value.detail), double(value.shadow), double(value.highlight),
            double(value.vibrance), double(value.saturation),
            smooth_->isChecked() ? 1.0 : 0.0, legacyCurve_, double(method_->currentIndex())};
}

bool MergePreview::eventFilter(QObject* object, QEvent* event)
{
    if (object == previewScroll_->viewport() && event->type() == QEvent::Resize) {
        QTimer::singleShot(0, this, &MergePreview::displayPreview);
    }
    return QDialog::eventFilter(object, event);
}

void MergePreview::displayPreview()
{
    if (previewImage_.isNull()) {
        image_->clear();
        return;
    }
    const int percent = zoom_->currentData().toInt();
    const bool fit = percent == 0;
    previewScroll_->setWidgetResizable(fit);
    const QSize size = fit ? previewScroll_->viewport()->size() : previewImage_.size() * percent / 100;
    if (size.isEmpty()) {
        return;
    }
    const QPixmap pixmap = QPixmap::fromImage(previewImage_.scaled(size, Qt::KeepAspectRatio,
                                                                 Qt::SmoothTransformation));
    if (!fit) {
        image_->resize(pixmap.size());
    }
    image_->setPixmap(pixmap);
}

void MergePreview::updatePreview()
{
    std::array<int, 256> bins{};
    previewImage_ = QImage();
    if (merged_ && hdr_result_error(**merged_).isEmpty()) {
        const auto values = toneValues();
        const auto lut = toneLut();
        previewImage_ = hdr_result_preview(bracket_, **merged_, asSlice(values), asSlice(lut),
                                           brightness_->value(), {bins.data(), bins.size()});
    }
    curve_->setBins(bins);
    displayPreview();
}

void MergePreview::rebuild()
{
    std::vector<double> exposures;
    exposures.reserve(included_.size());
    for (uint32_t index : included_) {
        exposures.push_back(ev100_[index]);
    }
    const int median = hdr_bracket_base(asSlice(exposures));
    const auto available = [this](int index) {
        return index >= 0 && std::find(included_.begin(), included_.end(), uint32_t(index))
                                     != included_.end();
    };
    const int selection = available(preferredBase_) ? preferredBase_
        : median >= 0 ? int(included_[size_t(median)]) : -1;
    {
        const QSignalBlocker block(base_);
        base_->setCurrentIndex(selection);
        base_->setEnabled(ghost_->isChecked());
        auto* model = qobject_cast<QStandardItemModel*>(base_->model());
        for (int index = 0; index < base_->count(); ++index) {
            model->item(index)->setEnabled(available(index));
            base_->setItemData(index, ghost_->isChecked() && index == selection
                ? QVariant(QColor(Qt::darkGreen)) : QVariant(), Qt::ForegroundRole);
        }
    }
    const int base = ghost_->isChecked() ? selection : -1;
    merged_.reset();
    merged_.emplace(hdr_bracket_merge(bracket_, asSlice(ev100_), {included_.data(), included_.size()},
                                      align_->isChecked(), base, asSlice(response_)));
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
    const auto lut = toneLut();
    auto toned = hdr_result_tone(bracket_, **merged_, asSlice(values), asSlice(lut));
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
            QJsonArray points;
            for (const QPointF point : curve_->points()) {
                points.append(QJsonArray{point.x(), point.y()});
            }
            data.insert(QStringLiteral("tonePoints"), points);
            data.insert(QStringLiteral("toneCorner"), corner_->isChecked());
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
        QVector<QPointF> points{QPointF(0, 0), QPointF(1, 1)};
        bool corner = false;
        if (data.contains(QStringLiteral("tonePoints"))) {
            const auto saved = data.value(QStringLiteral("tonePoints")).toArray();
            const auto savedCorner = data.value(QStringLiteral("toneCorner"));
            if (saved.size() < 2 || saved.size() > CurveWidget::kMaxPoints || !savedCorner.isBool()) {
                reject();
                return;
            }
            points.clear();
            for (const QJsonValue entry : saved) {
                const auto pair = entry.toArray();
                if (pair.size() != 2 || !pair.at(0).isDouble() || !pair.at(1).isDouble()) {
                    reject();
                    return;
                }
                const QPointF point(pair.at(0).toDouble(), pair.at(1).toDouble());
                if (point.x() < 0.0 || point.x() > 1.0 || point.y() < 0.0 || point.y() > 1.0
                    || (!points.isEmpty() && point.x() <= points.constLast().x())) {
                    reject();
                    return;
                }
                points.append(point);
            }
            if (points.constFirst().x() != 0.0 || points.constLast().x() != 1.0) {
                reject();
                return;
            }
            corner = savedCorner.toBool();
        }
        const auto tone = data.value(QStringLiteral("tone")).toArray();
        if (data.contains(QStringLiteral("tone")) && tone.size() != 12) {
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
        const QSignalBlocker blockSmooth(smooth_);
        const QSignalBlocker blockMethod(method_);
        const QSignalBlocker blockCorner(corner_);
        const QSignalBlocker blockCurve(curve_);
        legacyCurve_ = 0.0;
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
            legacyCurve_ = std::clamp(tone[10].toDouble(), -1.0, 1.0);
            method_->setCurrentIndex(std::clamp(tone[11].toInt(), 0, method_->count() - 1));
        }
        corner_->setChecked(corner);
        curve_->setLinearInterpolation(corner);
        curve_->setPoints(points);
        showSelectedPoint();
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
    bool accepted;
    bool installed;
    {
        MergePreview preview(&frame, *bracket, paths, ev, picker.alignImages(), *target);
        accepted = preview.exec() == QDialog::Accepted;
        installed = accepted && preview.install(*target);
    }
    if (!installed) {
        frame.closeDocument(index, false);
        if (previous >= 0) {
            frame.setActiveDocumentIndex(previous);
        }
        if (accepted) {
            QMessageBox::warning(&frame, tr("Merge To HDR Pro"), tr("Could not apply the HDR merge."));
        }
        return;
    }
    frame.refresh();
}

} // namespace pictura
