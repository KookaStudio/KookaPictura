// Decides whether a request over the socket may run, and holds it while it waits
// for a person or for a document to load. In-process dispatch() skips all this.

#include "control_server.h"
#include "control_server_support.h"

#include <QtCore/QFileInfo>
#include <QtCore/QTimer>
#include <QtNetwork/QLocalSocket>
#include <QtWidgets/QApplication>
#include <QtWidgets/QDialog>

#include "api/document_api.h"
#include "control_approval_bar.h"
#include "frame.h"
#include "tools.h"

namespace pictura {

using control_detail::pathProblem;

namespace {

bool isDocumentOp(const QString& method, const QJsonObject& params, const char* op)
{
    return method == QStringLiteral("document")
        && params.value(QStringLiteral("op")).toString() == QLatin1String(op);
}

bool isSave(const QString& method, const QJsonObject& params)
{
    return isDocumentOp(method, params, "save") || isDocumentOp(method, params, "save_as");
}

bool writesFile(const QString& method, const QJsonObject& params)
{
    return isSave(method, params) || method == QStringLiteral("export_png");
}

// `ownCrop`: the pending crop is the agent's, so it is not the person's to finish.
QString humanBusyReason(PicturaMainWindow* frame, bool ownCrop)
{
    if (QApplication::activeModalWidget() || QApplication::activePopupWidget()) {
        return QStringLiteral("a dialog or menu is open");
    }
    for (QWidget* widget : QApplication::topLevelWidgets()) {
        if (qobject_cast<QDialog*>(widget) && widget->isVisible()
            && widget->objectName() != QStringLiteral("jobBusy")) {
            return QStringLiteral("a dialog is open");
        }
    }
    if (QApplication::mouseButtons() != Qt::NoButton) {
        return QStringLiteral("a mouse button is held");
    }
    if (frame->hasPendingCrop() && !ownCrop) {
        return QStringLiteral("a crop is pending");
    }
    if (auto* tools = frame->findChild<ToolController*>(); tools && tools->transformPending()) {
        return QStringLiteral("a transform is pending");
    }
    for (int i = 0; i < frame->documentCount(); ++i) {
        if (frame->isLoadingDocument(i)) {
            return QStringLiteral("a document is loading");
        }
    }
    return {};
}

} // namespace

ControlApprovalBar* ControlServer::approvalBar() const
{
    return bar_.data();
}

void ControlServer::setPolicy(const ControlPolicy& policy)
{
    policy_ = policy;
}

ControlTier ControlServer::tierOf(const QString& method, const QJsonObject& params) const
{
    if (method == QStringLiteral("document")) {
        const QString op = params.value(QStringLiteral("op")).toString();
        if (op == QStringLiteral("save") || op == QStringLiteral("save_as")) {
            return ControlTier::Save;
        }
        if (op == QStringLiteral("new") || op == QStringLiteral("revert")
            || op == QStringLiteral("close")) {
            return ControlTier::Edit;
        }
        return ControlTier::Read;
    }
    if (method == QStringLiteral("history")
        && params.value(QStringLiteral("op")).toString() == QStringLiteral("list")) {
        return ControlTier::Read;
    }
    if (method == QStringLiteral("pointer")) {
        return params.value(QStringLiteral("space")).toString() == QStringLiteral("image")
            ? ControlTier::Edit
            : ControlTier::Admin;
    }
    for (const MethodRow& row : methodTable()) {
        if (method == QLatin1String(row.name)) {
            return row.tier;
        }
    }
    return ControlTier::Admin;
}

QJsonObject ControlServer::admit(const QString& method, const QJsonObject& params)
{
    if (!isMethod(method)) {
        return {};
    }
    const ControlTier tier = tierOf(method, params);
    if (!policy_.grants(tier)) {
        return error(QStringLiteral("refused"),
                     QStringLiteral("this session is not allowed to do that: %1").arg(method));
    }
    if (method == QStringLiteral("dispatch_command")
        && !commandAllowed(params.value(QStringLiteral("id")).toString())) {
        return error(QStringLiteral("refused"),
                     QStringLiteral("command is not available to agents: %1")
                         .arg(params.value(QStringLiteral("id")).toString()));
    }
    if (!policy_.live) {
        return {};
    }
    const bool discards = isDocumentOp(method, params, "close")
        || isDocumentOp(method, params, "revert");
    if (discards) {
        const int index = params.value(QStringLiteral("index")).toInt(frame_->activeDocumentIndex());
        if (index >= 0 && index < frame_->documentCount() && frame_->isDocumentDirty(index)) {
            return error(QStringLiteral("unsaved"),
                         QStringLiteral("the document has unsaved changes"));
        }
    }
    if (tier != ControlTier::Read) {
        if (const QString reason = humanBusyReason(frame_, agentCrop_); !reason.isEmpty()) {
            return error(QStringLiteral("human_busy"),
                         QStringLiteral("waiting for the person: %1").arg(reason));
        }
    }
    return {};
}

QString ControlServer::fileTarget(const QString& method, const QJsonObject& params) const
{
    const bool open = isDocumentOp(method, params, "open");
    if (!open && !writesFile(method, params)) {
        return {};
    }
    QString path = params.value(QStringLiteral("path")).toString();
    if (path.isEmpty() && isDocumentOp(method, params, "save")) {
        path = frame_->activeFilePath();
    }
    // The method answers these itself, as `invalid_param`.
    if (path.isEmpty() || !pathProblem(path, !open).isEmpty()) {
        return {};
    }
    return canonicalTarget(path);
}

PathVerdict ControlServer::verdictFor(const QString& method, const QJsonObject& params,
                                      const QString& target) const
{
    return judgePath(policy_, target, writesFile(method, params), QFileInfo::exists(target),
                     known_.contains(target));
}

void ControlServer::reply(const QPointer<QLocalSocket>& socket, int id, QJsonObject response)
{
    response.insert(QStringLiteral("id"), id);
    writeLine(socket, response);
}

void ControlServer::serve(const QPointer<QLocalSocket>& socket, int id, const QString& method,
                          const QJsonObject& params)
{
    if (isMethod(method) && tierOf(method, params) != ControlTier::Read
        && params.contains(QStringLiteral("doc"))) {
        PictureView* view = frame_->activeView();
        if (!view
            || params.value(QStringLiteral("doc")).toString() != control_detail::documentId(view)) {
            reply(socket, id,
                  error(QStringLiteral("stale_document"),
                        QStringLiteral("that document is not the active one")));
            return;
        }
    }
    if (policy_.restricted) {
        if (const QJsonObject refusal = admit(method, params); !refusal.isEmpty()) {
            reply(socket, id, refusal);
            return;
        }
        if (const QString target = fileTarget(method, params); !target.isEmpty()) {
            switch (verdictFor(method, params, target)) {
            case PathVerdict::Refuse:
                reply(socket, id,
                      error(QStringLiteral("refused"),
                            QStringLiteral("this session may not use %1").arg(target)));
                return;
            case PathVerdict::Ask:
                park(socket, id, method, params, target);
                return;
            case PathVerdict::Allow:
                break;
            }
        }
    }
    run(socket, id, method, params);
}

void ControlServer::run(const QPointer<QLocalSocket>& socket, int id, const QString& method,
                        const QJsonObject& params)
{
    const QString path = params.value(QStringLiteral("path")).toString();
    if (isDocumentOp(method, params, "open") && !path.isEmpty()
        && pathProblem(path, false).isEmpty()) {
        // Waits by parking, not in a nested loop: `interactive` off raises no
        // dialog, so there is nothing to guard, and a person's own is left alone.
        const quint64 token = frame_->startOpenDocumentAtPath(path, false);
        if (token == 0) {
            reply(socket, id,
                  error(QStringLiteral("io_error"), QStringLiteral("could not open %1").arg(path)));
            return;
        }
        opening_ = {socket, id, path, token};
        parked_ = true;
        return;
    }
    const bool cropBefore = frame_->hasPendingCrop();
    const QJsonObject response = dispatch(method, params);
    if (method == QStringLiteral("pointer") && !cropBefore && frame_->hasPendingCrop()) {
        agentCrop_ = true;
    }
    if (!frame_->hasPendingCrop()) {
        agentCrop_ = false;
    }
    if (writesFile(method, params) && response.value(QStringLiteral("ok")).toBool()) {
        known_.insert(canonicalTarget(isSave(method, params) ? frame_->activeFilePath() : path));
    }
    reply(socket, id, response);
}

void ControlServer::onOpenFinished(quint64 token, int outcome)
{
    if (token == 0 || token != opening_.token) {
        return;
    }
    const OpeningRequest finished = opening_;
    opening_ = {};
    parked_ = false;
    if (static_cast<PicturaMainWindow::OpenOutcome>(outcome)
        == PicturaMainWindow::OpenOutcome::Opened) {
        reply(finished.socket, finished.id, documentResult(true));
    } else {
        const bool cancelled = static_cast<PicturaMainWindow::OpenOutcome>(outcome)
            == PicturaMainWindow::OpenOutcome::Cancelled;
        reply(finished.socket, finished.id,
              error(cancelled ? QStringLiteral("cancelled") : QStringLiteral("io_error"),
                    QStringLiteral("could not open %1").arg(finished.path)));
    }
    QMetaObject::invokeMethod(this, &ControlServer::pump, Qt::QueuedConnection);
}

void ControlServer::park(const QPointer<QLocalSocket>& socket, int id, const QString& method,
                         const QJsonObject& params, const QString& target)
{
    pending_ = {socket, id, method, params, target};
    parked_ = true;
    if (!bar_) {
        bar_ = new ControlApprovalBar(frame_);
        connect(bar_, &ControlApprovalBar::answered, this, &ControlServer::resolveApproval);
    }
    if (!approvalTimer_) {
        approvalTimer_ = new QTimer(this);
        approvalTimer_->setSingleShot(true);
        connect(approvalTimer_, &QTimer::timeout, this, &ControlServer::onApprovalTimeout);
    }
    QString question;
    if (isDocumentOp(method, params, "open")) {
        question = tr("An agent wants to open %1");
    } else if (QFileInfo::exists(target)) {
        question = tr("An agent wants to overwrite %1");
    } else {
        question = tr("An agent wants to save to %1");
    }
    bar_->ask(question.arg(target));
    approvalTimer_->start(approvalTimeoutMs_);
}

ControlServer::PendingApproval ControlServer::takePending()
{
    if (approvalTimer_) {
        approvalTimer_->stop();
    }
    if (bar_) {
        bar_->hide();
    }
    PendingApproval pending = pending_;
    pending_ = {};
    parked_ = false;
    return pending;
}

void ControlServer::onApprovalTimeout()
{
    const PendingApproval pending = takePending();
    reply(pending.socket, pending.id,
          error(QStringLiteral("approval_timeout"), QStringLiteral("nobody answered")));
    QMetaObject::invokeMethod(this, &ControlServer::pump, Qt::QueuedConnection);
}

void ControlServer::resolveApproval(bool allow)
{
    if (pending_.target.isEmpty()) {
        return;
    }
    const PendingApproval pending = takePending();
    // As in pump(): the run below can spin a nested event loop.
    running_ = true;
    if (!allow) {
        reply(pending.socket, pending.id,
              error(QStringLiteral("refused"), QStringLiteral("the person declined")));
    } else if (const QJsonObject refusal = admit(pending.method, pending.params);
               !refusal.isEmpty()) {
        reply(pending.socket, pending.id, refusal);
    } else if (fileTarget(pending.method, pending.params) != pending.target
               || verdictFor(pending.method, pending.params, pending.target)
                   == PathVerdict::Refuse) {
        reply(pending.socket, pending.id,
              error(QStringLiteral("refused"), QStringLiteral("the target changed")));
    } else {
        known_.insert(pending.target);
        run(pending.socket, pending.id, pending.method, pending.params);
    }
    running_ = false;
    QMetaObject::invokeMethod(this, &ControlServer::pump, Qt::QueuedConnection);
}

} // namespace pictura
