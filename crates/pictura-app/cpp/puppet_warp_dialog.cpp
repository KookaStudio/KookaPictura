#include "puppet_warp_dialog.h"

#include "api/transform_bridge.h"
#include "dialogs.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QEvent>
#include <QtCore/QPointer>
#include <QtCore/QSignalBlocker>
#include <QtCore/QTimer>
#include <QtGui/QKeyEvent>
#include <QtGui/QKeySequence>
#include <QtGui/QMouseEvent>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtGui/QPolygon>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDialog>
#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QPushButton>
#include <QtWidgets/QSpinBox>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>
#include <limits>
#include <optional>
#include <utility>
#include <vector>

namespace pictura {
namespace {

constexpr int kMaximumPins = 32;
constexpr int kPadding = 48;
constexpr double kRingRadius = 21.0;
constexpr double kPickRadius = 8.0;
constexpr double kPi = 3.14159265358979323846;
constexpr double kDegrees = 180.0 / kPi;
constexpr double kPreviewPixels = 1024.0 * 1024.0;

double length(QPointF point)
{
    return std::hypot(point.x(), point.y());
}

struct WarpPin {
    QPointF rest;
    QPointF target;
    std::optional<double> angle;
    unsigned ordinal;
    bool selected = false;
};

struct Pins {
    QList<WarpPin> items;
    std::optional<unsigned> active;
};

class WarpCanvas final : public QWidget {
public:
    explicit WarpCanvas(QWidget* parent) : QWidget(parent)
    {
        setObjectName(QStringLiteral("puppetWarpCanvas"));
        setAccessibleName(tr("Puppet Warp preview"));
        setFocusPolicy(Qt::StrongFocus);
        setMouseTracking(true);
        setMinimumSize(270, 240);
    }

    void setSourceSize(int width, int height) { sourceSize_ = QSize(width, height); }
    double scale() const
    {
        return std::min(double(width() - 2 * kPadding) / sourceSize_.width(),
                        double(height() - 2 * kPadding) / sourceSize_.height());
    }
    QPointF canvasPosition(QPointF source) const { return offset() + source * scale(); }
    QPointF imagePosition(QPointF screen) const { return (screen - offset()) / scale(); }

    void present(QImage image, const rust::Vec<double>& mesh)
    {
        if (!image.isNull()) {
            preview_ = std::move(image);
        }
        overlay_.clear();
        for (std::size_t i = 0; i + 5 < mesh.size(); i += 6) {
            QPolygonF triangle;
            for (std::size_t j = i; j < i + 6; j += 2) {
                triangle << QPointF(mesh[j], mesh[j + 1]);
            }
            overlay_.append(triangle);
        }
        update();
    }

    const Pins* pins = nullptr;
    bool showMesh = true;
    bool hidePins = false;
    std::optional<unsigned> ring;

protected:
    void paintEvent(QPaintEvent*) override
    {
        QPainter painter(this);
        const auto theme = palette();
        QPixmap checker(20, 20);
        checker.fill(theme.color(QPalette::Base));
        {
            QPainter squares(&checker);
            squares.fillRect(0, 0, 10, 10, theme.color(QPalette::Mid));
            squares.fillRect(10, 10, 10, 10, theme.color(QPalette::Mid));
        }
        painter.fillRect(rect(), QBrush(checker));
        if (!preview_.isNull()) {
            painter.drawImage(rect(), preview_);
        }
        painter.setRenderHint(QPainter::Antialiasing);
        if (showMesh) {
            QColor light = theme.color(QPalette::Text);
            light.setAlpha(170);
            painter.setPen(QPen(theme.color(QPalette::Base), 2));
            drawMesh(painter);
            painter.setPen(QPen(light, 1));
            drawMesh(painter);
        }
        if (hidePins || !pins) {
            return;
        }
        for (const auto& pin : pins->items) {
            const QPointF at = canvasPosition(pin.target);
            if (ring == pin.ordinal) {
                painter.setPen(QPen(theme.color(QPalette::Highlight), 2, Qt::DashLine));
                painter.setBrush(Qt::NoBrush);
                painter.drawEllipse(at, kRingRadius, kRingRadius);
            }
            painter.setPen(QPen(theme.color(QPalette::Base), 2));
            painter.setBrush(theme.color(pin.selected ? QPalette::Highlight : QPalette::Text));
            const double radius = pins->active == pin.ordinal ? 7.0 : 5.0;
            painter.drawEllipse(at, radius, radius);
        }
    }

private:
    QPointF offset() const
    {
        return {(width() - sourceSize_.width() * scale()) * 0.5,
                (height() - sourceSize_.height() * scale()) * 0.5};
    }

    void drawMesh(QPainter& painter) const
    {
        for (const auto& triangle : overlay_) {
            QPolygonF mapped;
            for (const auto& point : triangle) {
                mapped << canvasPosition(point);
            }
            painter.drawPolygon(mapped);
        }
    }

    QSize sourceSize_{1, 1};
    QImage preview_;
    QList<QPolygonF> overlay_;
};

class WarpDialog final : public QDialog {
    Q_DECLARE_TR_FUNCTIONS(WarpDialog)

public:
    WarpDialog(QWidget* parent, PictureView& view, rust::Box<PuppetWarpSession> session,
               int width, int height)
        : QDialog(parent), view_(&view), session_(std::move(session))
    {
        setObjectName(QStringLiteral("puppetWarpDialog"));
        setWindowTitle(tr("Puppet Warp"));
        auto* root = new QVBoxLayout(this);
        auto* main = new QHBoxLayout;
        canvas_ = new WarpCanvas(this);
        canvas_->setSourceSize(width, height);
        canvas_->pins = &pins_;
        main->addWidget(canvas_, 1);
        main->addLayout(makeControls());
        root->addLayout(main, 1);

        status_ = new QLabel(this);
        status_->setObjectName(QStringLiteral("puppetWarpStatus"));
        status_->setWordWrap(true);
        root->addWidget(status_);

        auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
        apply_ = buttons->button(QDialogButtonBox::Ok);
        apply_->setText(tr("Apply"));
        connect(buttons, &QDialogButtonBox::accepted, this, &WarpDialog::apply);
        connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
        root->addWidget(buttons);
        canvas_->installEventFilter(this);
        for (auto* widget : findChildren<QWidget*>()) {
            if (widget != canvas_) {
                widget->installEventFilter(this);
            }
        }

        previewTimer_.setSingleShot(true);
        previewTimer_.setInterval(16);
        connect(&previewTimer_, &QTimer::timeout, this, [this]() { rebuild(); });
        setFocusProxy(canvas_);
        canvas_->setFocus();
        syncControls();
        resize(760, 520);
        queuePreview();
    }

    bool changed() const { return changed_; }

protected:
    bool eventFilter(QObject* object, QEvent* event) override
    {
        if (object != canvas_) {
            // Leave line-edit keys to spin boxes and other text controls.
            if (!qobject_cast<QLineEdit*>(object) && event->type() == QEvent::KeyPress) {
                if (key(*static_cast<QKeyEvent*>(event))) {
                    return true;
                }
            }
            if (event->type() == QEvent::KeyRelease) {
                const auto& k = *static_cast<QKeyEvent*>(event);
                if (k.key() == Qt::Key_H && canvas_->hidePins && !k.isAutoRepeat()) {
                    canvas_->hidePins = false;
                    canvas_->update();
                    return true;
                }
            }
            return QDialog::eventFilter(object, event);
        }
        switch (event->type()) {
        case QEvent::MouseButtonPress:
            return press(*static_cast<QMouseEvent*>(event));
        case QEvent::MouseMove:
            return move(*static_cast<QMouseEvent*>(event));
        case QEvent::MouseButtonRelease:
            return release(*static_cast<QMouseEvent*>(event));
        case QEvent::KeyPress:
            return key(*static_cast<QKeyEvent*>(event));
        case QEvent::KeyRelease: {
            const auto& k = *static_cast<QKeyEvent*>(event);
            if (k.key() == Qt::Key_H && !k.isAutoRepeat()) {
                canvas_->hidePins = false;
                canvas_->update();
                return true;
            }
            if (k.key() == Qt::Key_Alt) {
                setRing({});
            }
            break;
        }
        case QEvent::Leave:
            if (drag_ != Drag::Rotate) {
                setRing({});
            }
            break;
        case QEvent::FocusOut:
            canvas_->hidePins = false;
            setRing({});
            finishDrag();
            canvas_->update();
            break;
        case QEvent::Resize:
            queuePreview();
            break;
        default:
            break;
        }
        return QDialog::eventFilter(object, event);
    }

private:
    enum class Drag { None, Move, Rotate };

    QFormLayout* makeControls()
    {
        auto* controls = new QFormLayout;
        auto combo = [this, controls](const QString& name, const QString& label,
                                     const QStringList& items) {
            auto* box = new QComboBox(this);
            box->setObjectName(name);
            box->addItems(items);
            controls->addRow(label, box);
            return box;
        };
        mode_ = combo(QStringLiteral("puppetWarpMode"), tr("Mode"),
                      {tr("Rigid"), tr("Normal"), tr("Distort")});
        density_ = combo(QStringLiteral("puppetWarpDensity"), tr("Density"),
                         {tr("Fewer Points"), tr("Normal"), tr("More Points")});
        mode_->setCurrentIndex(1);
        density_->setCurrentIndex(1);
        expansion_ = new QSpinBox(this);
        expansion_->setObjectName(QStringLiteral("puppetWarpExpansion"));
        expansion_->setRange(0, 256);
        expansion_->setSuffix(tr(" px"));
        controls->addRow(tr("Expansion"), expansion_);

        auto* mesh = new QCheckBox(tr("Show Mesh"), this);
        mesh->setObjectName(QStringLiteral("puppetWarpShowMesh"));
        mesh->setChecked(true);
        controls->addRow(mesh);
        connect(mesh, &QCheckBox::toggled, this, [this](bool show) {
            canvas_->showMesh = show;
            canvas_->update();
        });

        rotate_ = combo(QStringLiteral("puppetWarpRotateMode"), tr("Rotate"),
                        {tr("Auto"), tr("Fixed")});
        angle_ = new QDoubleSpinBox(this);
        angle_->setObjectName(QStringLiteral("puppetWarpAngle"));
        angle_->setRange(-180.0, 180.0);
        angle_->setSuffix(QStringLiteral("°"));
        angle_->setDecimals(1);
        angle_->setKeyboardTracking(false);
        controls->addRow(tr("Angle"), angle_);
        auto* order = new QHBoxLayout;
        forward_ = new QPushButton(tr("Forward"), this);
        backward_ = new QPushButton(tr("Backward"), this);
        forward_->setObjectName(QStringLiteral("puppetWarpForward"));
        backward_->setObjectName(QStringLiteral("puppetWarpBackward"));
        forward_->setAutoDefault(false);
        backward_->setAutoDefault(false);
        order->addWidget(backward_);
        order->addWidget(forward_);
        controls->addRow(tr("Pin Depth"), order);

        connect(mode_, &QComboBox::currentIndexChanged, this, [this]() { queuePreview(); });
        connect(density_, &QComboBox::currentIndexChanged, this, [this]() { queuePreview(); });
        connect(expansion_, &QSpinBox::valueChanged, this, [this]() { queuePreview(); });
        connect(rotate_, &QComboBox::currentIndexChanged, this, [this](int index) {
            setAngle(index == 0 ? std::nullopt : std::optional<double>(angle_->value() / kDegrees));
        });
        connect(angle_, &QDoubleSpinBox::valueChanged, this,
                [this](double degrees) { setAngle(degrees / kDegrees); });
        connect(forward_, &QPushButton::clicked, this, [this]() { reorder(1); });
        connect(backward_, &QPushButton::clicked, this, [this]() { reorder(-1); });
        return controls;
    }

    qsizetype activeIndex() const
    {
        for (qsizetype i = 0; i < pins_.items.size(); ++i) {
            if (pins_.items[i].ordinal == pins_.active) {
                return i;
            }
        }
        return -1;
    }

    void syncControls()
    {
        const qsizetype i = activeIndex();
        const auto angle = i >= 0 ? pins_.items[i].angle : std::nullopt;
        const QSignalBlocker blockMode(rotate_);
        const QSignalBlocker blockAngle(angle_);
        rotate_->setEnabled(i >= 0);
        rotate_->setCurrentIndex(angle.has_value() ? 1 : 0);
        angle_->setEnabled(i >= 0 && angle.has_value());
        angle_->setValue(angle.value_or(0.0) * kDegrees);
        forward_->setEnabled(i >= 0 && i + 1 < pins_.items.size());
        backward_->setEnabled(i > 0);
        canvas_->update();
    }

    void remember(Pins before)
    {
        constexpr qsizetype kUndoLimit = 32;
        if (past_.size() == kUndoLimit) {
            past_.removeFirst();
        }
        past_.append(std::move(before));
    }

    void setAngle(std::optional<double> angle)
    {
        const auto i = activeIndex();
        if (i < 0 || pins_.items[i].angle == angle) {
            return;
        }
        remember(pins_);
        pins_.items[i].angle = angle;
        syncControls();
        queuePreview();
    }

    void reorder(int direction)
    {
        const auto i = activeIndex();
        const auto next = i + direction;
        if (i < 0 || next < 0 || next >= pins_.items.size()) {
            return;
        }
        remember(pins_);
        pins_.items.swapItemsAt(i, next);
        syncControls();
        queuePreview();
    }

    qsizetype hitPin(QPointF at, bool ringOnly) const
    {
        for (qsizetype i = pins_.items.size(); i-- > 0;) {
            const double distance = length(at - canvas_->canvasPosition(pins_.items[i].target));
            if (ringOnly ? std::abs(distance - kRingRadius) <= 6.0 : distance <= kPickRadius) {
                return i;
            }
        }
        return -1;
    }

    void setRing(std::optional<unsigned> ordinal)
    {
        if (canvas_->ring != ordinal) {
            canvas_->ring = ordinal;
            canvas_->update();
        }
    }

    void select(qsizetype index, bool extend)
    {
        if (!extend) {
            for (auto& pin : pins_.items) {
                pin.selected = false;
            }
        }
        pins_.active.reset();
        if (index >= 0) {
            auto& pin = pins_.items[index];
            pin.selected = true;
            pins_.active = pin.ordinal;
        }
        syncControls();
    }

    bool press(const QMouseEvent& event)
    {
        if (event.button() != Qt::LeftButton || canvas_->hidePins) {
            return false;
        }
        finishDrag();
        canvas_->setFocus();
        const QPointF at = event.position();
        auto hit = hitPin(at, false);
        if (event.modifiers().testFlag(Qt::AltModifier)) {
            if (hit >= 0) {
                remember(pins_);
                pins_.items.removeAt(hit);
                select(-1, false);
                setRing({});
                queuePreview();
                return true;
            }
            hit = hitPin(at, true);
            if (hit < 0) {
                return true;
            }
            select(hit, false);
            drag_ = Drag::Rotate;
            beforeDrag_ = pins_;
            rebuild();
            ringAngle_ = puppet_warp_pin_angle(*session_, pins_.items[hit].ordinal);
            ringStart_ = at - canvas_->canvasPosition(pins_.items[hit].target);
            setRing(pins_.active);
            return true;
        }

        const bool extend = event.modifiers().testFlag(Qt::ShiftModifier);
        beforeDrag_ = pins_;
        if (hit < 0) {
            if (pins_.items.size() >= kMaximumPins) {
                status_->setText(tr("The mesh supports up to 32 pins."));
                beforeDrag_.reset();
                return true;
            }
            rebuild();
            const QPointF point = canvas_->imagePosition(at);
            const auto hitSource = puppet_warp_hit(*session_, point.x(), point.y());
            if (hitSource.size() != 2) {
                select(-1, extend);
                beforeDrag_.reset();
                return true;
            }
            const QPointF rest(hitSource[0], hitSource[1]);
            const bool duplicate = std::any_of(pins_.items.cbegin(), pins_.items.cend(),
                [rest](const WarpPin& pin) { return length(pin.rest - rest) < 0.01; });
            if (duplicate) {
                beforeDrag_.reset();
                return true;
            }
            pins_.items.append(WarpPin{rest, point, {}, nextOrdinal_++});
            hit = pins_.items.size() - 1;
            dragChanged_ = true;
        }
        select(hit, extend);
        start_ = canvas_->imagePosition(at);
        dragStartPins_ = pins_.items;
        drag_ = Drag::Move;
        queuePreview();
        return true;
    }

    bool move(const QMouseEvent& event)
    {
        const QPointF at = event.position();
        if (drag_ == Drag::None) {
            const auto i = event.modifiers().testFlag(Qt::AltModifier) && !canvas_->hidePins
                ? hitPin(at, true) : -1;
            setRing(i >= 0 ? std::optional<unsigned>(pins_.items[i].ordinal) : std::nullopt);
            return false;
        }
        if (drag_ == Drag::Rotate) {
            const auto i = activeIndex();
            if (i < 0) {
                return true;
            }
            const auto direction = at - canvas_->canvasPosition(pins_.items[i].target);
            const double delta = std::atan2(direction.y(), direction.x())
                - std::atan2(ringStart_.y(), ringStart_.x());
            const double angle = ringAngle_ + delta;
            if (std::abs(delta) > 1e-6 || dragChanged_) {
                pins_.items[i].angle = std::remainder(angle, 2.0 * kPi);
                dragChanged_ = true;
                syncControls();
            }
        } else {
            const auto delta = canvas_->imagePosition(at) - start_;
            if (length(delta) > 1e-6 || dragChanged_) {
                for (qsizetype i = 0; i < pins_.items.size(); ++i) {
                    if (pins_.items[i].selected) {
                        pins_.items[i].target = dragStartPins_[i].target + delta;
                    }
                }
                dragChanged_ = true;
            }
        }
        queuePreview();
        return true;
    }

    bool release(const QMouseEvent& event)
    {
        if (event.button() != Qt::LeftButton || drag_ == Drag::None) {
            return false;
        }
        move(event);
        finishDrag();
        return true;
    }

    void finishDrag()
    {
        if (dragChanged_ && beforeDrag_) {
            remember(std::move(*beforeDrag_));
        }
        beforeDrag_.reset();
        dragStartPins_.clear();
        dragChanged_ = false;
        drag_ = Drag::None;
    }

    bool key(const QKeyEvent& event)
    {
        if (event.key() == Qt::Key_H) {
            canvas_->hidePins = true;
            canvas_->update();
            return true;
        }
        if (drag_ != Drag::None) {
            return false;
        }
        if (event.key() == Qt::Key_Delete || event.key() == Qt::Key_Backspace) {
            if (activeIndex() >= 0) {
                remember(pins_);
                pins_.items.removeIf([](const WarpPin& pin) { return pin.selected; });
                select(-1, false);
                setRing({});
                queuePreview();
            }
            return true;
        }
        if (event.matches(QKeySequence::Undo)) {
            if (!past_.isEmpty()) {
                pins_ = past_.takeLast();
                setRing({});
                syncControls();
                queuePreview();
            }
            return true;
        }
        if (event.matches(QKeySequence::SelectAll)) {
            for (auto& pin : pins_.items) {
                pin.selected = true;
            }
            if (!pins_.active && !pins_.items.isEmpty()) {
                pins_.active = pins_.items.constLast().ordinal;
            }
            syncControls();
            return true;
        }
        if (event.key() == Qt::Key_D && event.modifiers().testFlag(Qt::ControlModifier)) {
            select(-1, false);
            return true;
        }
        return false;
    }

    void queuePreview()
    {
        if (!previewTimer_.isActive()) {
            previewTimer_.start();
        }
        if (canvas_) {
            canvas_->update();
        }
    }

    void rebuild()
    {
        previewTimer_.stop();
        if (!apply_) {
            return;
        }
        std::vector<double> packed;
        packed.reserve(std::size_t(pins_.items.size()) * 6);
        for (const auto& pin : pins_.items) {
            packed.insert(packed.end(), {
                pin.rest.x(), pin.rest.y(), pin.target.x(), pin.target.y(),
                pin.angle.value_or(std::numeric_limits<double>::quiet_NaN()),
                double(pin.ordinal),
            });
        }
        const auto state = puppet_warp_configure(*session_,
            rust::Slice<const double>(packed.data(), packed.size()), mode_->currentIndex(),
            density_->currentIndex(), expansion_->value());
        const double area = double(canvas_->width()) * canvas_->height();
        const double reduction = std::min(1.0, std::sqrt(kPreviewPixels / area));
        const int w = std::max(1, int(canvas_->width() * reduction));
        const int h = std::max(1, int(canvas_->height() * reduction));
        const auto origin = canvas_->imagePosition({});
        auto image = puppet_warp_render(*session_, origin.x(), origin.y(),
                                       canvas_->scale() * reduction, w, h);
        ready_ = state > 0 && !image.isNull();
        changed_ = state == 2;
        canvas_->present(std::move(image), puppet_warp_triangles(*session_));
        apply_->setEnabled(ready_);
        status_->setText(ready_ ? QString() : tr("This mesh exceeds the preview or output limits."));
    }

    void apply()
    {
        finishDrag();
        rebuild();
        if (ready_ && view_ && puppet_warp_commit(*view_, *session_)) {
            accept();
        } else if (ready_) {
            status_->setText(tr("Puppet Warp could not apply. The target may have changed."));
        }
    }

    QPointer<PictureView> view_;
    rust::Box<PuppetWarpSession> session_;
    WarpCanvas* canvas_ = nullptr;
    QComboBox* mode_ = nullptr;
    QComboBox* density_ = nullptr;
    QComboBox* rotate_ = nullptr;
    QSpinBox* expansion_ = nullptr;
    QDoubleSpinBox* angle_ = nullptr;
    QPushButton* forward_ = nullptr;
    QPushButton* backward_ = nullptr;
    QPushButton* apply_ = nullptr;
    QLabel* status_ = nullptr;
    QTimer previewTimer_;
    Pins pins_;
    QList<Pins> past_;
    std::optional<Pins> beforeDrag_;
    QList<WarpPin> dragStartPins_;
    QPointF start_;
    QPointF ringStart_;
    double ringAngle_ = 0.0;
    unsigned nextOrdinal_ = 0;
    Drag drag_ = Drag::None;
    bool dragChanged_ = false;
    bool changed_ = false;
    bool ready_ = false;
};

} // namespace

bool runPuppetWarpDialog(QWidget* parent, PictureView& view)
{
    auto session = puppet_warp_begin(view);
    const auto size = puppet_warp_size(*session);
    if (size.size() != 2) {
        return false;
    }
    WarpDialog dialog(parent, view, std::move(session), size[0], size[1]);
    return runDialog(dialog, parent) == QDialog::Accepted && dialog.changed();
}

} // namespace pictura
