#include "color_range_dialog.h"
#include <QtCore/QCoreApplication>

#include "icons.h"
#include "panels/jump_slider.h"
#include "tools.h"

#include "api/selection_bridge.h"

#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QSignalBlocker>
#include <QtCore/QStringList>
#include <QtGui/QKeyEvent>
#include <QtGui/QPalette>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QFileDialog>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QRadioButton>
#include <QtWidgets/QSlider>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>

namespace pictura {

namespace {

// Longest side of the preview mask/image.
constexpr int kPreviewSize = 220;

QStringList selectNames()
{
    return {QStringLiteral("Sampled Colors"), QStringLiteral("Skin Tones"),
            QStringLiteral("Reds"),           QStringLiteral("Yellows"),
            QStringLiteral("Greens"),         QStringLiteral("Cyans"),
            QStringLiteral("Blues"),          QStringLiteral("Magentas"),
            QStringLiteral("Highlights"),     QStringLiteral("Midtones"),
            QStringLiteral("Shadows")};
}

} // namespace

ColorRangeDialog::ColorRangeDialog(PictureView* view, ToolController* tools, QWidget* parent)
    : QDialog(parent)
    , view_(view)
    , tools_(tools)
{
    setWindowTitle(QStringLiteral("Color Range"));
    setObjectName(QStringLiteral("colorRangeDialog"));
    auto* root = new QVBoxLayout(this);
    auto* form = new QGridLayout();

    form->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Select:"), this), 0, 0);
    select_ = new QComboBox(this);
    select_->setObjectName(QStringLiteral("colorRangeSelect"));
    const QStringList names = selectNames();
    for (int i = 0; i < names.size(); ++i) {
        select_->addItem(names.at(i), i);
        if (i == 1 || i == 7) {
            select_->insertSeparator(select_->count());
        }
    }
    if (view_ && color_range_out_of_gamut_available(*view_)) {
        select_->insertSeparator(select_->count());
        select_->addItem(QCoreApplication::translate("PicturaUi", "Out Of Gamut"), 11);
    }
    form->addWidget(select_, 0, 1, 1, 3);

    form->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Fuzziness:"), this), 1, 0);
    fuzziness_ = new QSpinBox(this);
    fuzziness_->setObjectName(QStringLiteral("colorRangeFuzziness"));
    fuzziness_->setRange(0, 200);
    fuzziness_->setValue(40);
    form->addWidget(fuzziness_, 1, 1);
    fuzzinessSlider_ = new JumpSlider(Qt::Horizontal, this);
    fuzzinessSlider_->setRange(0, 200);
    fuzzinessSlider_->setValue(40);
    form->addWidget(fuzzinessSlider_, 2, 0, 1, 4);

    form->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Color:"), this), 3, 0);
    swatch_ = new QLabel(this);
    swatch_->setObjectName(QStringLiteral("colorRangeSwatch"));
    swatch_->setFixedSize(44, 18);
    swatch_->setFrameShape(QFrame::Box);
    swatch_->setAutoFillBackground(true);
    form->addWidget(swatch_, 3, 1);
    auto* droppers = new QHBoxLayout();
    auto* group = new QButtonGroup(this);
    group->setExclusive(true);
    const auto makeDropper = [&](const QString& name, const QString& tip) {
        auto* button = new QToolButton(this);
        button->setObjectName(name);
        button->setCheckable(true);
        button->setIcon(icon(QStringLiteral("tool.eyedropper")));
        button->setToolTip(tip);
        group->addButton(button);
        droppers->addWidget(button);
        return button;
    };
    replaceButton_ = makeDropper(QStringLiteral("colorRangeReplace"),
                                 QStringLiteral("Sample the color to match"));
    addButton_ = makeDropper(QStringLiteral("colorRangeAdd"),
                             QStringLiteral("Add to sample"));
    subtractButton_ = makeDropper(QStringLiteral("colorRangeSubtract"),
                                  QStringLiteral("Subtract from sample"));
    replaceButton_->setChecked(true);
    form->addLayout(droppers, 3, 2, 1, 2);

    form->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Range:"), this), 4, 0);
    range_ = new QSpinBox(this);
    range_->setObjectName(QStringLiteral("colorRangeRange"));
    range_->setRange(0, 100);
    range_->setValue(100);
    form->addWidget(range_, 4, 1);
    rangeSlider_ = new JumpSlider(Qt::Horizontal, this);
    rangeSlider_->setRange(0, 100);
    rangeSlider_->setValue(100);
    form->addWidget(rangeSlider_, 5, 0, 1, 4);
    localized_ = new QCheckBox(QCoreApplication::translate("PicturaUi", "Localized Color Clusters"), this);
    localized_->setObjectName(QStringLiteral("colorRangeLocalized"));
    form->addWidget(localized_, 6, 0, 1, 4);
    root->addLayout(form);

    invert_ = new QCheckBox(QCoreApplication::translate("PicturaUi", "Invert"), this);
    invert_->setObjectName(QStringLiteral("colorRangeInvert"));
    root->addWidget(invert_);

    auto* displayRow = new QHBoxLayout();
    displayRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Display:"), this));
    selectionButton_ = new QRadioButton(QCoreApplication::translate("PicturaUi", "Selection"), this);
    selectionButton_->setObjectName(QStringLiteral("colorRangeSelection"));
    imageButton_ = new QRadioButton(QCoreApplication::translate("PicturaUi", "Image"), this);
    imageButton_->setObjectName(QStringLiteral("colorRangeImage"));
    selectionButton_->setChecked(true);
    displayRow->addWidget(selectionButton_);
    displayRow->addWidget(imageButton_);
    displayRow->addStretch();
    root->addLayout(displayRow);

    previewLabel_ = new QLabel(this);
    previewLabel_->setAlignment(Qt::AlignCenter);
    previewLabel_->setMinimumSize(kPreviewSize, kPreviewSize / 2);
    previewLabel_->setFrameShape(QFrame::Box);
    root->addWidget(previewLabel_, 1);

    auto* previewRow = new QHBoxLayout();
    previewRow->addWidget(new QLabel(QCoreApplication::translate("PicturaUi", "Selection Preview:"), this));
    previewMode_ = new QComboBox(this);
    previewMode_->setObjectName(QStringLiteral("colorRangeSelectionPreview"));
    const QStringList previews = {
        QStringLiteral("None"),        QStringLiteral("Grayscale"),
        QStringLiteral("Black Matte"), QStringLiteral("White Matte"),
        QStringLiteral("Quick Mask")};
    for (int i = 0; i < previews.size(); ++i) {
        previewMode_->addItem(previews.at(i), i);
    }
    previewRow->addWidget(previewMode_, 1);
    root->addLayout(previewRow);

    auto* saveLoad = new QHBoxLayout();
    auto* saveButton = new QPushButton(QCoreApplication::translate("PicturaUi", "Save..."), this);
    saveButton->setObjectName(QStringLiteral("colorRangeSave"));
    auto* loadButton = new QPushButton(QCoreApplication::translate("PicturaUi", "Load..."), this);
    loadButton->setObjectName(QStringLiteral("colorRangeLoad"));
    saveLoad->addWidget(saveButton);
    saveLoad->addWidget(loadButton);
    saveLoad->addStretch();
    root->addLayout(saveLoad);
    status_ = new QLabel(this);
    status_->setObjectName(QStringLiteral("colorRangeStatus"));
    status_->setWordWrap(true);
    root->addWidget(status_);

    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);

    // Start on the foreground colour, usually the one just sampled (as CS6).
    if (tools_) {
        samples_.append({QPoint(-1, -1), tools_->foreground()});
    }

    connect(select_, &QComboBox::currentIndexChanged, this, [this]() {
        refreshEnabled();
        refreshSwatches();
        refreshPreview();
        refreshCanvasPreview();
    });
    connect(fuzzinessSlider_, &QSlider::valueChanged, fuzziness_, &QSpinBox::setValue);
    connect(fuzziness_, QOverload<int>::of(&QSpinBox::valueChanged), fuzzinessSlider_,
            &QSlider::setValue);
    connect(fuzziness_, QOverload<int>::of(&QSpinBox::valueChanged), this, [this]() {
        refreshPreview();
        refreshCanvasPreview();
    });
    connect(localized_, &QCheckBox::toggled, this, [this]() {
        refreshEnabled();
        refreshPreview();
        refreshCanvasPreview();
    });
    connect(rangeSlider_, &QSlider::valueChanged, range_, &QSpinBox::setValue);
    connect(range_, QOverload<int>::of(&QSpinBox::valueChanged), rangeSlider_,
            &QSlider::setValue);
    connect(range_, QOverload<int>::of(&QSpinBox::valueChanged), this, [this]() {
        refreshPreview();
        refreshCanvasPreview();
    });
    connect(invert_, &QCheckBox::toggled, this, [this]() {
        refreshPreview();
        refreshCanvasPreview();
    });
    connect(selectionButton_, &QRadioButton::toggled, this, [this](bool on) {
        if (on) {
            refreshPreview();
        }
    });
    connect(imageButton_, &QRadioButton::toggled, this, [this](bool on) {
        if (on) {
            refreshPreview();
        }
    });
    connect(previewMode_, &QComboBox::currentIndexChanged, this,
            [this]() { refreshCanvasPreview(); });
    connect(replaceButton_, &QToolButton::clicked, this, [this]() { mode_ = PickMode::Replace; });
    connect(addButton_, &QToolButton::clicked, this, [this]() { mode_ = PickMode::Add; });
    connect(subtractButton_, &QToolButton::clicked, this, [this]() {
        mode_ = PickMode::Subtract;
    });
    connect(saveButton, &QPushButton::clicked, this, &ColorRangeDialog::chooseSave);
    connect(loadButton, &QPushButton::clicked, this, &ColorRangeDialog::chooseLoad);

    refreshEnabled();
    refreshSwatches();
    refreshPreview();
    refreshCanvasPreview();
}

ColorRangeDialog::~ColorRangeDialog()
{
    if (tools_) {
        tools_->setCanvasSampler({});
    }
    if (view_) {
        color_range_preview_clear(*view_);
    }
}

int ColorRangeDialog::range() const
{
    return select_->currentData().toInt();
}

QColor ColorRangeDialog::sampledColor() const
{
    return samples_.isEmpty() ? QColor(Qt::black) : samples_.last().color;
}

QString ColorRangeDialog::samplesString() const
{
    QStringList parts;
    parts.reserve(samples_.size());
    for (const Sample& s : samples_) {
        parts << QStringLiteral("%1,%2,%3,%4,%5")
                     .arg(s.pos.x())
                     .arg(s.pos.y())
                     .arg(s.color.red())
                     .arg(s.color.green())
                     .arg(s.color.blue());
    }
    return parts.join(QLatin1Char(';'));
}

int ColorRangeDialog::fuzziness() const
{
    return fuzziness_->value();
}

bool ColorRangeDialog::localized() const
{
    return localized_->isChecked();
}

int ColorRangeDialog::localizedRange() const
{
    return range_->value();
}

bool ColorRangeDialog::inverted() const
{
    return invert_->isChecked();
}

int ColorRangeDialog::displayMode() const
{
    return imageButton_->isChecked() ? 1 : 0;
}

int ColorRangeDialog::selectionPreview() const
{
    return previewMode_->currentData().toInt();
}

void ColorRangeDialog::sampleAt(const QPointF& imagePos, Qt::KeyboardModifiers modifiers)
{
    if (!view_ || range() != 0) {
        return;
    }
    const QPoint p = imagePos.toPoint();
    if (p.x() < 0 || p.y() < 0 || p.x() >= view_->document_width()
        || p.y() >= view_->document_height()) {
        return;
    }
    const QColor color = QColor::fromRgb(view_->sample_argb(p.x(), p.y()));
    PickMode picked = mode_;
    if (modifiers.testFlag(Qt::ShiftModifier)) {
        picked = PickMode::Add;
    } else if (modifiers.testFlag(Qt::AltModifier)) {
        picked = PickMode::Subtract;
    }
    switch (picked) {
    case PickMode::Replace:
        samples_.clear();
        samples_.append({p, color});
        break;
    case PickMode::Add:
        samples_.append({p, color});
        break;
    case PickMode::Subtract: {
        // Drop the samples this click falls within, so clicking a colour an
        // earlier sample pulled in takes it back out again.
        const int tolerance = fuzziness();
        for (int i = samples_.size() - 1; i >= 0; --i) {
            const Sample& s = samples_.at(i);
            if (s.pos.x() < 0 || s.pos.y() < 0) {
                continue;
            }
            const int distance =
                qMax(qMax(qAbs(s.color.red() - color.red()),
                          qAbs(s.color.green() - color.green())),
                     qAbs(s.color.blue() - color.blue()));
            if (distance <= tolerance) {
                samples_.removeAt(i);
            }
        }
        break;
    }
    }
    refreshSwatches();
    refreshPreview();
    refreshCanvasPreview();
}

void ColorRangeDialog::hideEvent(QHideEvent* event)
{
    if (tools_) {
        tools_->setCanvasSampler({});
    }
    if (view_) {
        color_range_preview_clear(*view_);
    }
    QDialog::hideEvent(event);
}

void ColorRangeDialog::keyPressEvent(QKeyEvent* event)
{
    // Ctrl / Cmd toggles the dialog's Selection and Image display, as CS6.
    if (event->key() == Qt::Key_Control || event->key() == Qt::Key_Meta) {
        if (displayMode() == 0) {
            imageButton_->setChecked(true);
        } else {
            selectionButton_->setChecked(true);
        }
        event->accept();
        return;
    }
    QDialog::keyPressEvent(event);
}

void ColorRangeDialog::refreshSampler()
{
    if (!tools_) {
        return;
    }
    if (view_ && range() == 0) {
        tools_->setCanvasSampler(
            [this](const QPointF& p, Qt::KeyboardModifiers modifiers) {
                sampleAt(p, modifiers);
            },
            false);
    } else {
        tools_->setCanvasSampler({});
    }
}

void ColorRangeDialog::refreshPreview()
{
    if (!view_) {
        return;
    }
    if (displayMode() == 1) {
        imagePreview_ = color_range_preview_image(*view_, kPreviewSize);
        preview_ = QImage();
        if (imagePreview_.isNull()) {
            previewLabel_->clear();
        } else {
            previewLabel_->setPixmap(QPixmap::fromImage(imagePreview_)
                                         .scaled(previewLabel_->size(),
                                                 Qt::KeepAspectRatio,
                                                 Qt::SmoothTransformation));
        }
        return;
    }
    int width = 0;
    int height = 0;
    const ::rust::Vec<uint8_t> coverage = color_range_preview(
        *view_, range(), samplesString(), fuzziness(), localized(), localizedRange(), inverted(),
        kPreviewSize, width, height);
    if (width <= 0 || height <= 0 || int(coverage.size()) != width * height) {
        preview_ = QImage();
        imagePreview_ = QImage();
        previewLabel_->clear();
        return;
    }
    QImage image(width, height, QImage::Format_Grayscale8);
    for (int y = 0; y < height; ++y) {
        std::copy_n(coverage.data() + size_t(y) * size_t(width), width, image.scanLine(y));
    }
    preview_ = image;
    imagePreview_ = QImage();
    previewLabel_->setPixmap(QPixmap::fromImage(image).scaled(
        previewLabel_->size(), Qt::KeepAspectRatio, Qt::SmoothTransformation));
}

void ColorRangeDialog::refreshCanvasPreview()
{
    if (!view_) {
        return;
    }
    color_range_preview_set(*view_, range(), samplesString(), fuzziness(), localized(),
                            localizedRange(), inverted(), selectionPreview());
}

void ColorRangeDialog::refreshEnabled()
{
    // A sample is meaningful only for Sampled Colors; bands, Skin Tones, and
    // Out Of Gamut are defined by their rule.
    const bool sampled = range() == 0;
    swatch_->setEnabled(sampled);
    replaceButton_->setEnabled(sampled);
    addButton_->setEnabled(sampled);
    subtractButton_->setEnabled(sampled);
    const bool clusters = localized_->isChecked();
    range_->setEnabled(clusters);
    rangeSlider_->setEnabled(clusters);
    refreshSampler();
}

void ColorRangeDialog::refreshSwatches()
{
    const QColor color = samples_.isEmpty() ? QColor(Qt::transparent) : samples_.last().color;
    QPalette palette = swatch_->palette();
    palette.setColor(QPalette::Window, swatch_->isEnabled() ? color : QColor(Qt::transparent));
    swatch_->setPalette(palette);
}

QString ColorRangeDialog::selectNameFor(int index)
{
    switch (index) {
    case 0:
        return QStringLiteral("sampled");
    case 1:
        return QStringLiteral("skin-tones");
    case 2:
        return QStringLiteral("reds");
    case 3:
        return QStringLiteral("yellows");
    case 4:
        return QStringLiteral("greens");
    case 5:
        return QStringLiteral("cyans");
    case 6:
        return QStringLiteral("blues");
    case 7:
        return QStringLiteral("magentas");
    case 8:
        return QStringLiteral("highlights");
    case 9:
        return QStringLiteral("midtones");
    case 10:
        return QStringLiteral("shadows");
    case 11:
        return QStringLiteral("out-of-gamut");
    default:
        return {};
    }
}

int ColorRangeDialog::indexForSelectName(const QString& name)
{
    static const QStringList names = {
        QStringLiteral("sampled"),     QStringLiteral("skin-tones"),
        QStringLiteral("reds"),        QStringLiteral("yellows"),
        QStringLiteral("greens"),      QStringLiteral("cyans"),
        QStringLiteral("blues"),       QStringLiteral("magentas"),
        QStringLiteral("highlights"),  QStringLiteral("midtones"),
        QStringLiteral("shadows"),     QStringLiteral("out-of-gamut")};
    return int(names.indexOf(name));
}

bool ColorRangeDialog::saveSettings(const QString& path)
{
    QJsonObject root;
    root.insert(QStringLiteral("format"), QStringLiteral("pictura-color-range"));
    root.insert(QStringLiteral("version"), 1);
    root.insert(QStringLiteral("select"), selectNameFor(range()));
    root.insert(QStringLiteral("fuzziness"), fuzziness());
    root.insert(QStringLiteral("localized"), localized());
    root.insert(QStringLiteral("range"), localizedRange());
    root.insert(QStringLiteral("invert"), inverted());
    QJsonArray array;
    for (const Sample& s : samples_) {
        QJsonObject sample;
        sample.insert(QStringLiteral("x"), s.pos.x());
        sample.insert(QStringLiteral("y"), s.pos.y());
        sample.insert(QStringLiteral("r"), s.color.red());
        sample.insert(QStringLiteral("g"), s.color.green());
        sample.insert(QStringLiteral("b"), s.color.blue());
        array.append(sample);
    }
    root.insert(QStringLiteral("samples"), array);
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        settingsError_ = QStringLiteral("The settings file could not be opened for writing.");
        return false;
    }
    if (file.write(QJsonDocument(root).toJson(QJsonDocument::Indented)) < 0) {
        settingsError_ = QStringLiteral("The settings file could not be written.");
        return false;
    }
    settingsError_.clear();
    return true;
}

bool ColorRangeDialog::loadSettings(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        settingsError_ = QStringLiteral("The settings file could not be read.");
        return false;
    }
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
        settingsError_ = QStringLiteral("The settings file is not valid JSON.");
        return false;
    }
    const QJsonObject root = document.object();
    if (root.value(QStringLiteral("format")).toString()
            != QLatin1String("pictura-color-range")
        || root.value(QStringLiteral("version")).toInt() != 1) {
        settingsError_ = QStringLiteral("The settings file has an unsupported format.");
        return false;
    }
    const int index = indexForSelectName(root.value(QStringLiteral("select")).toString());
    if (index < 0 || select_->findData(index) < 0) {
        settingsError_ = QStringLiteral("The settings file selects an unavailable mode.");
        return false;
    }
    if (index == 11 && view_ && !color_range_out_of_gamut_available(*view_)) {
        settingsError_ = QStringLiteral("Out Of Gamut is not available on this document.");
        return false;
    }
    QVector<Sample> samples;
    for (const QJsonValue& value : root.value(QStringLiteral("samples")).toArray()) {
        if (!value.isObject()) {
            continue;
        }
        const QJsonObject sample = value.toObject();
        const int x = sample.value(QStringLiteral("x")).toInt(-1);
        const int y = sample.value(QStringLiteral("y")).toInt(-1);
        const int r = qBound(0, sample.value(QStringLiteral("r")).toInt(), 255);
        const int g = qBound(0, sample.value(QStringLiteral("g")).toInt(), 255);
        const int b = qBound(0, sample.value(QStringLiteral("b")).toInt(), 255);
        samples.append({QPoint(x, y), QColor(r, g, b)});
    }
    const int fuzz = qBound(0, root.value(QStringLiteral("fuzziness")).toInt(40), 200);
    const bool clusters = root.value(QStringLiteral("localized")).toBool(false);
    const int range = qBound(0, root.value(QStringLiteral("range")).toInt(100), 100);
    const bool invert = root.value(QStringLiteral("invert")).toBool(false);
    {
        const QSignalBlocker blockSelect(select_);
        const QSignalBlocker blockFuzziness(fuzziness_);
        const QSignalBlocker blockFuzzinessSlider(fuzzinessSlider_);
        const QSignalBlocker blockLocalized(localized_);
        const QSignalBlocker blockRange(range_);
        const QSignalBlocker blockRangeSlider(rangeSlider_);
        const QSignalBlocker blockInvert(invert_);
        select_->setCurrentIndex(select_->findData(index));
        fuzziness_->setValue(fuzz);
        fuzzinessSlider_->setValue(fuzz);
        localized_->setChecked(clusters);
        range_->setValue(range);
        rangeSlider_->setValue(range);
        invert_->setChecked(invert);
    }
    samples_ = samples;
    refreshEnabled();
    refreshSwatches();
    refreshPreview();
    refreshCanvasPreview();
    settingsError_.clear();
    return true;
}

void ColorRangeDialog::chooseSave()
{
    const QString path = QFileDialog::getSaveFileName(
        this, QStringLiteral("Save Color Range Settings"), QString(),
        QStringLiteral("Color Range Settings (*.json)"));
    if (path.isEmpty()) {
        return;
    }
    status_->setText(saveSettings(path) ? QString() : settingsError_);
}

void ColorRangeDialog::chooseLoad()
{
    const QString path = QFileDialog::getOpenFileName(
        this, QStringLiteral("Load Color Range Settings"), QString(),
        QStringLiteral("Color Range Settings (*.json)"));
    if (path.isEmpty()) {
        return;
    }
    status_->setText(loadSettings(path) ? QString() : settingsError_);
}

} // namespace pictura

#include "moc_color_range_dialog.cpp"
