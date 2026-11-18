#include "print_job.h"

#include "api/printing_bridge.h"
#include "api/proofing_bridge.h"
#include "session.h"

#include <QtCore/QCryptographicHash>
#include <QtCore/QDir>
#include <QtCore/QDirIterator>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QJsonDocument>
#include <QtCore/QProcess>
#include <QtCore/QRegularExpression>
#include <QtCore/QSaveFile>
#include <QtCore/QStandardPaths>
#include <QtCore/QTemporaryFile>
#include <QtGui/QPainter>
#include <QtPrintSupport/QPrinterInfo>

#include <algorithm>
#include <cstdint>

namespace pictura {

namespace {

constexpr qint64 maxIccSize = 16 * 1024 * 1024;
constexpr qint64 maxSettingsSize = 1024 * 1024;
const QStringList proofNames{QStringLiteral("proofCMYK"), QStringLiteral("Working Gray"),
                             QStringLiteral("sRGB IEC61966-2.1"), QStringLiteral("Adobe RGB (1998)"),
                             QStringLiteral("ProPhoto RGB")};

QByteArray bytes(const ::rust::Vec<std::uint8_t>& data)
{
    return QByteArray(reinterpret_cast<const char*>(data.data()), qsizetype(data.size()));
}

::rust::Slice<const std::uint8_t> slice(const QByteArray& data)
{
    return {reinterpret_cast<const std::uint8_t*>(data.constData()), std::size_t(data.size())};
}

::rust::Slice<const std::uint8_t> slice(const QImage& image)
{
    return {image.constBits(), std::size_t(image.sizeInBytes())};
}

QString json(const QJsonObject& object)
{
    return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Compact));
}

QString printStorePath(const QString& name)
{
    return QFileInfo(sessionFilePath()).dir().filePath(name);
}

QJsonObject readObject(const QString& path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly) || file.size() > maxSettingsSize) {
        return {};
    }
    return QJsonDocument::fromJson(file.readAll()).object();
}

bool saveBytes(const QString& path, const QByteArray& data)
{
    QSaveFile file(path);
    return file.open(QIODevice::WriteOnly) && file.write(data) == data.size() && file.commit();
}

bool saveObject(const QString& path, const QJsonObject& obj)
{
    return QDir().mkpath(QFileInfo(path).absolutePath())
        && saveBytes(path, QJsonDocument(obj).toJson(QJsonDocument::Compact));
}

QImage imageFromRaster(const PrintRaster& raster)
{
    if (!raster.error.isEmpty() || raster.samples.empty() || raster.components != 4) {
        return {};
    }
    return QImage(raster.samples.data(), int(raster.width), int(raster.height),
                  qsizetype(raster.width) * 4, QImage::Format_RGBA8888).copy();
}

bool managed(const PrintSettings& settings)
{
    return settings.options.value(QStringLiteral("managed")).toBool();
}

QString sha256(const QByteArray& data)
{
    return QString::fromLatin1(QCryptographicHash::hash(data, QCryptographicHash::Sha256).toHex());
}

QByteArray runCups(const QString& name, const QStringList& args)
{
#ifdef Q_OS_LINUX
    const QString executable = QStandardPaths::findExecutable(name);
    if (!executable.isEmpty()) {
        QProcess process;
        auto environment = QProcessEnvironment::systemEnvironment();
        environment.insert(QStringLiteral("LC_ALL"), QStringLiteral("C"));
        process.setProcessEnvironment(environment);
        process.start(executable, args);
        if (process.waitForStarted(2000) && process.waitForFinished(8000)
            && process.exitStatus() == QProcess::NormalExit && process.exitCode() == 0) {
            return process.readAllStandardOutput().trimmed();
        }
        if (process.state() != QProcess::NotRunning) {
            process.kill();
            process.waitForFinished();
        }
    }
#else
    Q_UNUSED(name);
    Q_UNUSED(args);
#endif
    return {};
}

} // namespace

QString printQueueFingerprint(const QString& queue, const PrintCupsCommand& command)
{
    const QByteArray device = command(QStringLiteral("lpstat"), {QStringLiteral("-v"), queue});
    const QByteArray defaults = command(QStringLiteral("lpoptions"), {QStringLiteral("-p"), queue});
    const QByteArray options = command(QStringLiteral("lpoptions"),
                                       {QStringLiteral("-p"), queue, QStringLiteral("-l")});
    if (device.isEmpty() || defaults.isEmpty() || options.isEmpty()) {
        return {};
    }
    return sha256(qgetenv("CUPS_SERVER") + '\0' + device + '\0' + defaults + '\0' + options);
}

namespace {

QString routePath(const QJsonObject& route)
{
    return printStorePath(QStringLiteral("print-routes/%1.json").arg(sha256(json(route).toUtf8())));
}

QJsonObject savedEnrollment(const PrintSettings& settings, const PrintProfile& profile)
{
    return readObject(routePath(managedPrintRoute(settings, profile)));
}

bool cupsOptions(const QJsonObject& evidence, QStringList* args)
{
    const QJsonObject options = evidence.value(QStringLiteral("options")).toObject();
    const QString orientation = evidence.value(QStringLiteral("orientation")).toString();
    if (orientation != QStringLiteral("pdf") && orientation != QStringLiteral("pdf-with-portrait-option")) {
        return false;
    }
    static const QRegularExpression token(QStringLiteral("^[A-Za-z0-9_.-]+$"));
    if (!options.contains(QStringLiteral("media"))) {
        return false;
    }
    const auto route = evidence.value(QStringLiteral("route")).toObject();
    if (options.value(QStringLiteral("media")) != route.value(QStringLiteral("page"))
        || (route.value(QStringLiteral("source")).toInt() != QPrinter::Auto
            && !options.contains(QStringLiteral("media-source")))) {
        return false;
    }
    for (auto it = options.begin(); it != options.end(); ++it) {
        const QString value = it.value().toString();
        if (it.key() == QStringLiteral("copies") || it.key() == QStringLiteral("orientation-requested")
            || it.key() == QStringLiteral("document-format") || it.key() == QStringLiteral("job-attribute-fidelity")
            || !token.match(it.key()).hasMatch() || !token.match(value).hasMatch()) {
            return false;
        }
        *args << QStringLiteral("-o") << (it.key() + QLatin1Char('=') + value);
    }
    if (orientation == QStringLiteral("pdf-with-portrait-option")) {
        *args << QStringLiteral("-o") << QStringLiteral("orientation-requested=3");
    }
    return true;
}

QString managedQueueHelp()
{
    return QStringLiteral("Application-managed printing requires a validated CUPS colour bypass "
                          "for this printer, paper, tray and profile. Choose Save as PDF or "
                          "Printer Manages Colors, or configure and verify this route.");
}

QString sampleOperators(const QByteArray& samples)
{
    if (samples.isEmpty()) {
        return {};
    }
    QString result = QStringLiteral("/Print cs\n");
    for (char sample : samples) {
        result += QString::number(double(std::uint8_t(sample)) / 255, 'f', 5) + QLatin1Char(' ');
    }
    return result + QStringLiteral("scn\n");
}

QString printColorOperators(const QColor& color, const PrintRaster& raster, const QString& settings)
{
    const std::uint8_t rgb[] = {std::uint8_t(color.red()), std::uint8_t(color.green()),
                                std::uint8_t(color.blue())};
    return sampleOperators(bytes(print_device_color(
        {rgb, 3}, {raster.profile.data(), raster.profile.size()}, settings)));
}

QByteArray pagePdf(const PrintPageLayout& page, const PrintRaster& raster, const QString& settings)
{
    const double geometry[] = {page.paper.width(), page.paper.height(), page.image.left(),
                               page.paper.height() - page.image.bottom(), page.image.width(),
                               page.image.height()};
    QString background;
    if (page.background.isValid()) {
        QPainterPath rect;
        rect.addRect(page.printable);
        background = printColorOperators(page.background, raster, settings)
            + printPathOperators(rect, page.paper.height());
    }
    QString marks;
    for (const PrintMark& mark : page.marks) {
        const QString color = raster.components == 4 && mark.deviceColor.size() == 4
            ? sampleOperators(mark.deviceColor) : printColorOperators(mark.color, raster, settings);
        marks += color + printPathOperators(mark.path, page.paper.height());
    }
    return bytes(print_pdf_bytes(raster, {geometry, 6}, background, marks));
}

} // namespace

PrintSettings readPrintSettings()
{
    const auto saved = readObject(printStorePath(QStringLiteral("print-settings.json")));
    PrintSettings result;
    result.printer = saved.value(QStringLiteral("printer")).toString(QPrinterInfo::defaultPrinter().printerName());
    result.copies = qBound(1, saved.value(QStringLiteral("copies")).toInt(1), 999);
    result.landscape = saved.value(QStringLiteral("landscape")).toBool();
    result.paperSource = QPrinter::PaperSource(saved.value(QStringLiteral("paperSource")).toInt(QPrinter::Auto));
    const QSizeF size(saved.value(QStringLiteral("width")).toDouble(), saved.value(QStringLiteral("height")).toDouble());
    if (!size.isEmpty()) {
        result.pageSize = QPageSize(size, QPageSize::Point, saved.value(QStringLiteral("pageName")).toString());
    }
    result.options = saved.value(QStringLiteral("options")).toObject();
    return result;
}

PrintSettings& lastPrintSettings()
{
    static PrintSettings settings = readPrintSettings();
    return settings;
}

bool storePrintSettings(const PrintSettings& settings)
{
    const QSizeF size = settings.pageSize.size(QPageSize::Point);
    const QJsonObject store{{QStringLiteral("printer"), settings.printer},
                            {QStringLiteral("copies"), settings.copies},
                            {QStringLiteral("landscape"), settings.landscape},
                            {QStringLiteral("paperSource"), int(settings.paperSource)},
                            {QStringLiteral("width"), size.width()}, {QStringLiteral("height"), size.height()},
                            {QStringLiteral("pageName"), settings.pageSize.name()},
                            {QStringLiteral("options"), settings.options}};
    const bool saved = saveObject(printStorePath(QStringLiteral("print-settings.json")), store);
    lastPrintSettings() = settings;
    return saved;
}

QByteArray profileBytes(const QString& path)
{
    if (path.startsWith(QStringLiteral("builtin:"))) {
        bool valid = false;
        const int device = path.mid(8).toInt(&valid);
        return valid ? bytes(print_builtin_profile(device)) : QByteArray();
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly) || file.size() <= 0 || file.size() > maxIccSize) {
        return {};
    }
    return file.readAll();
}

PrintProfile describePrintProfile(const QString& path)
{
    const QByteArray content = profileBytes(path);
    const auto info = QJsonDocument::fromJson(print_output_profile_info(slice(content)).toUtf8()).object();
    QString name = info.value(QStringLiteral("name")).toString();
    if (!name.isEmpty() && path.startsWith(QStringLiteral("builtin:"))) {
        bool valid = false;
        const int index = path.mid(8).toInt(&valid);
        if (valid && index > 0 && index < proofNames.size()) {
            name = proofNames.at(index);
        }
    }
    return {name, path,
            info.value(QStringLiteral("components")).toInt(), content};
}

QList<PrintProfile> installedPrintProfiles()
{
    QList<PrintProfile> profiles;
    for (int device = 0; device < proofNames.size(); ++device) {
        profiles.append(describePrintProfile(QStringLiteral("builtin:%1").arg(device)));
    }
    QStringList roots = QStandardPaths::locateAll(QStandardPaths::GenericDataLocation,
                                                 QStringLiteral("color/icc"), QStandardPaths::LocateDirectory);
    roots << QDir::home().filePath(QStringLiteral(".color/icc")) << QStringLiteral("/var/lib/colord/icc");
    roots.removeDuplicates();
    for (const QString& root : roots) {
        QDirIterator files(root, {QStringLiteral("*.icc"), QStringLiteral("*.icm")},
                           QDir::Files | QDir::Readable, QDirIterator::Subdirectories);
        while (files.hasNext()) {
            const PrintProfile profile = describePrintProfile(files.next());
            if (profile.components != 0 && std::none_of(profiles.begin(), profiles.end(),
                    [&](const PrintProfile& known) { return known.bytes == profile.bytes; })) {
                profiles.append(profile);
            }
        }
    }
    return profiles;
}

PrintProfile resolvePrintProfile(const QJsonObject& options)
{
    const QString path = options.value(QStringLiteral("profilePath")).toString();
    const QString name = options.value(QStringLiteral("profile")).toString();
    if (!path.isEmpty()) {
        const PrintProfile profile = describePrintProfile(path);
        if (profile.name == name || name.isEmpty()) {
            return profile;
        }
    }
    if (name.isEmpty()) {
        return {};
    }
    PrintProfile match;
    for (const PrintProfile& profile : installedPrintProfiles()) {
        if (profile.name == name) {
            if (match.components != 0) {
                return {};
            }
            match = profile;
        }
    }
    return match;
}

QByteArray resolvePrintProof(const QJsonObject& proof)
{
    const QString name = proof.value(QStringLiteral("name")).toString();
    const QString path = proof.value(QStringLiteral("path")).toString();
    if (!path.isEmpty()) {
        const auto selected = describePrintProfile(path);
        return selected.components != 0 ? selected.bytes : QByteArray();
    }
    const auto device = proofNames.indexOf(name);
    if (device >= 0 && proof.value(QStringLiteral("kind")).toString() == QStringLiteral("builtin")) {
        return profileBytes(QStringLiteral("builtin:%1").arg(device));
    }
    return resolvePrintProfile({{QStringLiteral("profile"), name},
                                {QStringLiteral("profilePath"), proof.value(QStringLiteral("path"))}}).bytes;
}

QJsonObject viewPrintProof(PictureView* view)
{
    const int device = view ? proof_device(*view) : 0;
    const QString path = QStringLiteral("builtin:%1").arg(device);
    const bool builtinCmyk = device == 0
        && (!view || (proof_intent(*view) == 1 && proof_bpc(*view) && !proof_simulate_paper(*view)));
    const QString name = device == 0 && !builtinCmyk
        ? describePrintProfile(path).name : proofNames.value(device);
    return {{QStringLiteral("name"), name}, {QStringLiteral("path"), path},
            {QStringLiteral("kind"), builtinCmyk ? QStringLiteral("builtin") : QStringLiteral("named")},
            {QStringLiteral("intent"), view ? proof_intent(*view) : 1},
            {QStringLiteral("bpc"), !view || proof_bpc(*view)},
            {QStringLiteral("paper"), view && proof_simulate_paper(*view)},
            {QStringLiteral("black"), view && proof_simulate_black(*view)}};
}

PrintSettings resolvedPrintSettings(PictureView* view)
{
    PrintSettings settings = lastPrintSettings();
    const auto localProof = settings.options.value(QStringLiteral("proof")).toObject();
    const QJsonObject document = view
        ? QJsonDocument::fromJson(print_document_settings(*view).toUtf8()).object() : QJsonObject();
    if (document.contains(QStringLiteral("profile"))
        && document.value(QStringLiteral("profile")) != settings.options.value(QStringLiteral("profile"))) {
        settings.options.remove(QStringLiteral("profilePath"));
        settings.options.remove(QStringLiteral("components"));
    }
    for (auto it = document.begin(); it != document.end(); ++it) {
        settings.options.insert(it.key(), it.value());
    }
    if (document.contains(QStringLiteral("proof"))) {
        auto proof = document.value(QStringLiteral("proof")).toObject();
        if (proof.value(QStringLiteral("name")) == localProof.value(QStringLiteral("name"))
            && proof.value(QStringLiteral("kind")) == localProof.value(QStringLiteral("kind"))) {
            proof.insert(QStringLiteral("black"), localProof.value(QStringLiteral("black")));
            proof.insert(QStringLiteral("path"), localProof.value(QStringLiteral("path")));
        }
        settings.options.insert(QStringLiteral("proof"), proof);
    } else {
        settings.options.insert(QStringLiteral("proof"), viewPrintProof(view));
    }
    return settings;
}

QPageLayout physicalPrintPage(const PrintSettings& settings)
{
    QPrinter printer;
    configurePrinter(printer, settings, QString());
    return printer.pageLayout();
}

QJsonObject managedPrintRoute(const PrintSettings& settings, const PrintProfile& profile)
{
    const QPageLayout page = physicalPrintPage(settings);
    const QSizeF size = page.pageSize().size(QPageSize::Point);
    return {{QStringLiteral("destination"), settings.printer},
            {QStringLiteral("page"), page.pageSize().key()},
            {QStringLiteral("pageWidth"), size.width()},
            {QStringLiteral("pageHeight"), size.height()},
            {QStringLiteral("landscape"), settings.landscape},
            {QStringLiteral("source"), int(settings.paperSource)},
            {QStringLiteral("profile"), sha256(profile.bytes)},
            {QStringLiteral("components"), profile.components}};
}

QJsonObject managedPrintEvidence(const PrintSettings& settings, const PrintProfile& profile)
{
    const auto saved = savedEnrollment(settings, profile);
    if (!saved.isEmpty()) {
        auto evidence = saved;
        evidence.remove(QStringLiteral("fingerprint"));
        return evidence;
    }
    const auto route = managedPrintRoute(settings, profile);
    const QJsonObject options{{QStringLiteral("media"), route.value(QStringLiteral("page"))}};
    return {{QStringLiteral("route"), route}, {QStringLiteral("device"), QString()},
            {QStringLiteral("filters"), QString()}, {QStringLiteral("evidence"), QString()},
            {QStringLiteral("orientation"), QString()},
            {QStringLiteral("options"), options}};
}

QString revokeManagedPrint(const PrintSettings& settings, const PrintProfile& profile)
{
    const QString path = routePath(managedPrintRoute(settings, profile));
    if (QFile::exists(path) && !QFile::remove(path)) {
        return QStringLiteral("The CUPS route could not be revoked.");
    }
    return {};
}

QString enrollManagedPrint(const PrintSettings& settings, const PrintProfile& profile,
                           const QJsonObject& evidence)
{
    if (settings.printer.isEmpty() || profile.components == 0) {
        return QStringLiteral("Select a native printer and an output profile first.");
    }
    const auto route = managedPrintRoute(settings, profile);
    QStringList args;
    if (evidence.value(QStringLiteral("route")).toObject() != route
        || evidence.value(QStringLiteral("evidence")).toString().trimmed().isEmpty()
        || evidence.value(QStringLiteral("device")).toString().trimmed().isEmpty()
        || evidence.value(QStringLiteral("filters")).toString().trimmed().isEmpty()
        || !cupsOptions(evidence, &args)
        || (settings.paperSource != QPrinter::Auto
            && !evidence.value(QStringLiteral("options")).toObject().contains(QStringLiteral("media-source")))) {
        return QStringLiteral("The route file must match this job and record the validated device, "
                              "filter chain, bypass evidence, media and tray options.");
    }
    const QString fingerprint = printQueueFingerprint(settings.printer, runCups);
    if (fingerprint.isEmpty()) {
        return managedQueueHelp();
    }
    QJsonObject registration = evidence;
    registration.insert(QStringLiteral("fingerprint"), fingerprint);
    if (!saveObject(routePath(route), registration)) {
        return QStringLiteral("The validated CUPS route could not be saved.");
    }
    return {};
}

PrintCupsJob prepareManagedCups(const PrintSettings& settings, const PrintProfile& profile,
                               const QJsonObject& enrollment, const PrintCupsCommand& command)
{
    PrintCupsJob job{settings.printer, managedPrintRoute(settings, profile), {}, {}};
    if (settings.printer.isEmpty() || profile.components == 0
        || enrollment.value(QStringLiteral("route")).toObject() != job.route
        || enrollment.value(QStringLiteral("evidence")).toString().trimmed().isEmpty()
        || enrollment.value(QStringLiteral("device")).toString().trimmed().isEmpty()
        || enrollment.value(QStringLiteral("filters")).toString().trimmed().isEmpty()) {
        job.error = managedQueueHelp();
        return job;
    }
    const QString fingerprint = printQueueFingerprint(settings.printer, command);
    if (fingerprint.isEmpty() || enrollment.value(QStringLiteral("fingerprint")).toString() != fingerprint
        || !cupsOptions(enrollment, &job.arguments)) {
        job.error = managedQueueHelp();
        return job;
    }
    job.arguments << QStringLiteral("-d") << settings.printer
                  << QStringLiteral("-n") << QString::number(settings.copies)
                  << QStringLiteral("-o") << QStringLiteral("document-format=application/pdf")
                  << QStringLiteral("-o") << QStringLiteral("job-attribute-fidelity=true");
    return job;
}

QString submitManagedCups(PrintCupsJob job, const QByteArray& pdf, const PrintCupsCommand& command)
{
    if (!job.error.isEmpty()) {
        return job.error;
    }
    QTemporaryFile file(QDir::temp().filePath(QStringLiteral("pictura-print-XXXXXX.pdf")));
    if (!file.open() || file.write(pdf) != pdf.size() || !file.flush()) {
        return QStringLiteral("The print job could not be staged for CUPS.");
    }
    file.close();
    job.arguments << QStringLiteral("--") << file.fileName();
    const QByteArray result = command(QStringLiteral("lp"), job.arguments);
    static const QRegularExpression jobId(QStringLiteral(R"(^request id is \S+-[0-9]+(?:\s|$))"));
    return jobId.match(QString::fromUtf8(result)).hasMatch() ? QString()
        : QStringLiteral("CUPS did not confirm a job ID. Check the queue before retrying.");
}

QString printJobPreflight(const PrintSettings& settings)
{
    if (!managed(settings)) {
        return {};
    }
    const auto profile = resolvePrintProfile(settings.options);
    if (profile.components == 0) {
        return QStringLiteral("Choose an available, unambiguous printer ICC profile.");
    }
    if (settings.options.value(QStringLiteral("hardProof")).toBool()
        && resolvePrintProof(settings.options.value(QStringLiteral("proof")).toObject()).isEmpty()) {
        return QStringLiteral("Choose an available ICC profile for the print proof.");
    }
    if (settings.printer.isEmpty()) {
        return {};
    }
    if (QStandardPaths::findExecutable(QStringLiteral("lp")).isEmpty()) {
        return managedQueueHelp();
    }
    return prepareManagedCups(settings, profile, savedEnrollment(settings, profile), runCups).error;
}

QString submitPrintJob(const PrintSettings& settings, PictureView* view, const QImage& image,
                       const QString& path, const QString& filename, const QString& description)
{
    const QString preflight = printJobPreflight(settings);
    if (!preflight.isEmpty()) {
        return preflight;
    }
    const QImage fallback = image.convertToFormat(QImage::Format_RGBA8888);
    const QSizeF size = view ? QSizeF(view->document_width(), view->document_height()) : QSizeF(fallback.size());
    const PrintPageLayout page = layoutPrintPage(physicalPrintPage(settings), size,
                                                settings.options, filename, description);
    if (page.image.isEmpty()) {
        return QStringLiteral("The printer has no usable printable area.");
    }
    if (!settings.printer.isEmpty() && !managed(settings)) {
        const QImage source = view ? printableImage(view) : fallback;
        if (source.isNull()) {
            return QStringLiteral("The image could not be prepared.");
        }
        QPrinter printer(QPrinter::HighResolution);
        configurePrinter(printer, settings, QString());
        printer.setFullPage(true);
        QPainter painter(&printer);
        if (painter.isActive()) {
            painter.scale(painter.viewport().width() / page.paper.width(),
                          painter.viewport().height() / page.paper.height());
            paintPrintPage(painter, page, source, Qt::white);
            return painter.end() ? QString()
                : QStringLiteral("The image could not be printed.");
        }
        return QStringLiteral("The printer could not start the job.");
    }
    const PrintProfile profile = managed(settings) ? resolvePrintProfile(settings.options) : PrintProfile();
    const QByteArray proof = managed(settings) && settings.options.value(QStringLiteral("hardProof")).toBool()
        ? resolvePrintProof(settings.options.value(QStringLiteral("proof")).toObject()) : QByteArray();
    const QString options = json(settings.options);
    const PrintRaster raster = view ? print_document_raster(*view, slice(profile.bytes), slice(proof), options)
        : print_image_raster(slice(fallback), fallback.width(), fallback.height(), slice(profile.bytes), slice(proof), options);
    if (!raster.error.isEmpty()) {
        return raster.error;
    }
    const QByteArray document = pagePdf(page, raster, options);
    if (document.isEmpty()) {
        return QStringLiteral("The print PDF could not be prepared.");
    }
    if (settings.printer.isEmpty()) {
        return saveBytes(path, document) ? QString() : QStringLiteral("The print PDF could not be saved.");
    }
    return submitManagedCups(
        prepareManagedCups(settings, profile, savedEnrollment(settings, profile), runCups), document, runCups);
}

QImage printPreviewSource(PictureView& view, int side)
{
    return imageFromRaster(print_source_preview(view, side));
}

QImage proofPrintPreview(const QImage& image, const PrintSettings& settings, QString& error)
{
    const bool hardProof = settings.options.value(QStringLiteral("hardProof")).toBool();
    if (!managed(settings) || (!hardProof && !settings.options.value(QStringLiteral("match")).toBool())) {
        return image;
    }
    const QByteArray output = resolvePrintProfile(settings.options).bytes;
    const QByteArray proof = hardProof ? resolvePrintProof(settings.options.value(QStringLiteral("proof")).toObject()) : QByteArray();
    if (output.isEmpty() || (hardProof && proof.isEmpty())) {
        error = output.isEmpty() ? QStringLiteral("Choose an available printer ICC profile.")
                                 : QStringLiteral("The print proof profile is unavailable. Showing unproofed colors.");
        return image;
    }
    const QImage rgba = image.convertToFormat(QImage::Format_RGBA8888);
    const auto preview = print_preview_pixels(slice(rgba), slice(output), slice(proof), json(settings.options));
    if (preview.size() != std::size_t(rgba.sizeInBytes())) {
        error = QStringLiteral("The print proof could not be prepared. Showing unproofed colors.");
        return image;
    }
    return QImage(preview.data(), rgba.width(), rgba.height(), rgba.bytesPerLine(), QImage::Format_RGBA8888).copy();
}

QImage proofDocumentPrintPreview(PictureView& view, const QImage& image,
                                 const PrintSettings& settings, QString& error)
{
    const bool proofed = managed(settings)
        && (settings.options.value(QStringLiteral("match")).toBool()
            || settings.options.value(QStringLiteral("hardProof")).toBool());
    const QByteArray output = proofed ? resolvePrintProfile(settings.options).bytes : QByteArray();
    const QByteArray proof = proofed && settings.options.value(QStringLiteral("hardProof")).toBool()
        ? resolvePrintProof(settings.options.value(QStringLiteral("proof")).toObject()) : QByteArray();
    if (proofed && (output.isEmpty()
        || (settings.options.value(QStringLiteral("hardProof")).toBool() && proof.isEmpty()))) {
        return proofPrintPreview(image, settings, error);
    }
    auto options = settings.options;
    options.insert(QStringLiteral("managed"), proofed);
    const auto raster = print_document_preview(view, qMax(image.width(), image.height()),
                                               slice(output), slice(proof), json(options));
    if (!raster.error.isEmpty()) {
        error = raster.error;
        return image;
    }
    QImage preview = imageFromRaster(raster);
    if (preview.isNull()) {
        error = QStringLiteral("The print preview could not be prepared.");
        return image;
    }
    preview = preview.scaled(image.size(), Qt::IgnoreAspectRatio, Qt::SmoothTransformation);
    return preview;
}

QColor printPaperAppearance(const PrintSettings& settings)
{
    if (!settings.options.value(QStringLiteral("paperWhite")).toBool()) {
        return Qt::white;
    }
    const QByteArray icc = managed(settings) ? resolvePrintProfile(settings.options).bytes : QByteArray();
    const auto paper = print_paper_color(slice(icc));
    return paper.size() == 3 ? QColor(paper[0], paper[1], paper[2]) : QColor(0xf5, 0xf5, 0xf0);
}

} // namespace pictura
