#pragma once

#include "print_dialog.h"
#include "print_page_layout.h"

#include <QtCore/QByteArray>

namespace pictura {

struct PrintProfile {
    QString name;
    QString path;
    int components = 0;
    QByteArray bytes;
};

QByteArray profileBytes(const QString& path);
PrintProfile describePrintProfile(const QString& path);
QList<PrintProfile> installedPrintProfiles();
PrintProfile resolvePrintProfile(const QJsonObject& options);
QByteArray resolvePrintProof(const QJsonObject& proof);
QJsonObject viewPrintProof(PictureView* view);
PrintSettings resolvedPrintSettings(PictureView* view);
bool storePrintSettings(const PrintSettings& settings);

struct PrintCupsJob {
    QString destination;
    QJsonObject route;
    QStringList arguments;
    QString error;
};

using PrintCupsCommand = std::function<QByteArray(const QString&, const QStringList&)>;
PrintCupsJob prepareManagedCups(const PrintSettings& settings, const PrintProfile& profile,
                               const QJsonObject& enrollment, const PrintCupsCommand& command);
QString submitManagedCups(PrintCupsJob job, const QByteArray& pdf, const PrintCupsCommand& command);
QString printQueueFingerprint(const QString& queue, const PrintCupsCommand& command);
QJsonObject managedPrintRoute(const PrintSettings& settings, const PrintProfile& profile);
QJsonObject managedPrintEvidence(const PrintSettings& settings, const PrintProfile& profile);
QString enrollManagedPrint(const PrintSettings& settings, const PrintProfile& profile,
                           const QJsonObject& evidence);
QString revokeManagedPrint(const PrintSettings& settings, const PrintProfile& profile);

QString printJobPreflight(const PrintSettings& settings);
QString submitPrintJob(const PrintSettings& settings, PictureView* view, const QImage& image,
                       const QString& path, const QString& filename, const QString& description);
QPageLayout physicalPrintPage(const PrintSettings& settings);
QImage printPreviewSource(PictureView& view, int side);
QImage proofPrintPreview(const QImage& image, const PrintSettings& settings, QString& error);
QImage proofDocumentPrintPreview(PictureView& view, const QImage& image,
                                 const PrintSettings& settings, QString& error);
QColor printPaperAppearance(const PrintSettings& settings);

void routePrintOneCopy(PrintSettings settings, bool available,
                       const std::function<QString(const PrintSettings&)>& preflight,
                       const std::function<void(const PrintSettings&)>& submit,
                       const std::function<void(const QString&)>& openDialog);

} // namespace pictura
