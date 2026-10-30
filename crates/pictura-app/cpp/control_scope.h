#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>

namespace pictura {

enum class UnsavedChoice;

// Spans one control request. A dialog or menu shown meanwhile would wait on a
// person, so it is cancelled and recorded; the request is then answered
// `refused`. Catches what `isModalCommand` does not list.
//
// ponytail: a native portal file dialog may never emit Show, so it can slip
// past; the denylist still names every known file-dialog command.
class ControlScope : public QObject {
public:
    // With `unsavedChoice`, an unsaved prompt reached by this request resolves
    // to it. Null leaves the process-wide policy in charge.
    explicit ControlScope(const UnsavedChoice* unsavedChoice = nullptr);
    ~ControlScope() override;

    // The title of the first dialog or menu cancelled, or empty.
    QString cancelled() const { return cancelled_; }
    bool cancelledAny() const { return cancelledAny_; }

protected:
    bool eventFilter(QObject* watched, QEvent* event) override;

private:
    QString cancelled_;
    bool cancelledAny_ = false;
};

} // namespace pictura
