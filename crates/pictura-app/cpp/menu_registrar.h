#pragma once

#include <QtGui/QColor>
#include <QtCore/QList>
#include <QtCore/QObject>
#include <QtCore/QPoint>
#include <QtCore/QPointer>
#include <QtCore/QString>
#include <QtCore/QStringList>

#include <functional>
#include <memory>

class QAction;
class QMenu;
class QMenuBar;
class QUrl;
class QWidget;

namespace pictura {

class ColorRangeDialog;
class CommandRegistry;
class DocWindow;
class DocumentRegistry;
class ImageView;
class ImportSaveCoordinator;
class OptionsBar;
class PanelHost;
class PictureView;
class ReplaceColorDialog;
class ToolController;
struct CommandParams;
struct FilterCommandSpec;

// Owns the command table and every command-handler/provider/context-menu
// registration, and builds the menu bar. The composition root supplies the
// chrome it still owns as a registration callback and the small runtime below,
// so the registrar never names the main window class.
class MenuRegistrar : public QObject {
    Q_OBJECT

public:
    struct Runtime {
        std::function<void()> requestRefresh;
        std::function<void()> openSaveSelectionDialog;
        std::function<void()> openLoadSelectionDialog;
        std::function<void()> openDuplicateChannelDialog;
        std::function<void(const QString& message, int timeoutMs)> showStatus;
        std::function<OptionsBar*()> optionsBar;
        std::function<QWidget*()> dialogParent;
        std::function<int()> uiScalePercent;
        std::function<QColor()> foregroundColor;
        std::function<void()> updateWindowTitle;
        std::function<void(const QUrl&)> openUrl;
        std::function<void()> installMenuCornerButtons;
        std::function<bool(DocWindow*, bool)> closeWindow;
    };

    MenuRegistrar(std::function<void(CommandRegistry&)> registerFrameCommands, Runtime runtime,
                  QObject* parent = nullptr);
    ~MenuRegistrar() override;

    void setCollaborators(DocumentRegistry* documents, PanelHost* panels,
                          ImportSaveCoordinator* importSave);
    void setTools(ToolController* tools) { tools_ = tools; }

    CommandRegistry* commands() const { return commands_; }
    DocumentRegistry* documents() const { return documents_; }
    PanelHost* panels() const { return panels_; }
    ImportSaveCoordinator* importSave() const { return importSave_; }
    ToolController* tools() const { return tools_; }

    void registerHandlers();
    void buildMenus(QMenuBar* menuBar);
    void wireDocumentMenus();
    void wireWindowArrange();
    void wireSmartObjectRowMenu();
    void wireWorkspaceCommands();

    // Command helpers shared with the frame's forwards and the panel hooks.
    bool beginFreeTransform(const QString& path);
    bool beginTransformMode(const QString& path, const QString& mode);
    void selectLayerPath(const QString& path);
    void editSmartFilter(const QString& path, int index);
    void convertImageMode(const QString& mode);
    void convertImageDepth(int bits);
    void cancelDocumentDialogs(PictureView* view);
    void importSystemClipboard();
    bool handleDeleteKey();
    bool replaceSmartObjectContents(const QString& layerPath);
    bool exportSmartObjectContents(const QString& layerPath);
    void alignSelectedLayers(int edge, bool toSelection, int alignTo = 0);
    void distributeSelectedLayers(int edge);
    void updateAlignControls();
    void updateUiSizeMenu();

    QStringList selectedLayerPaths() const;
    QString layerAnchor() const;
    QMenu* recentMenu() const;
    QMenu* documentMenu() const;

private:
    QWidget* dialogParent() const;
    QColor foregroundColor() const;
    OptionsBar* optionsBar() const;
    PictureView* activeView() const;
    ImageView* imageView() const;
    int documentCount() const;
    int activeDocumentIndex() const;
    PictureView* viewAt(int index) const;
    QString documentName(int index) const;
    QString activeDocumentName() const;
    QString activeFilePath() const;
    QList<ImageView*> activeDocumentCanvases() const;
    void refresh();
    void showStatus(const QString& message, int timeoutMs);
    void reportFilterRefusal(PictureView* view);

    void registerImageCommands();
    void registerLayerCommands();
    void registerSelectCommands();
    void registerEditCommands();
    void registerPanelCommands();
    void registerTextCommands();

    void registerEditHandlers();
    void registerSelectHandlers();
    void wireAlignMenu();
    void wireEditHistory();
    void wireFilterMenu();
    void wireImageMenu();
    void wireImageModeMenu();
    void wireImageAdjustments();
    void wireLayerAdjustments();
    void wireLayerContentOptions();
    void wireLayerArrangeStamp();
    void wireShapeLayerActions();
    void wireVectorMaskActions();
    void wireLayerStyleMenu();
    void wireMattingActions();
    void wireAdvancedSmartObjectActions();
    void wireSmartFilterActions();
    void wireTransformMenu();
    void wireTypeMenu();
    void wireWarpText();
    void wireReplaceMissingFonts();
    void wirePanelMenuCommands();
    void applyFilterCommand(const FilterCommandSpec& spec);
    bool runAutoAlignJob(int projection, bool vignette, bool geometric);
    bool runAutoBlendJob(bool stackImages, bool seamless, bool fillTransparent);
    void showTabMenu(DocWindow* window, const QPoint& barPos);
    void showHeaderMenu(DocWindow* window, const QPoint& globalPos);
    void consolidateAllToTabs(DocWindow* target);
    void closeOtherWindows(DocWindow* keep);
    void revealInFileManager(const DocWindow* window);
    void refreshRecentMenu(QMenu* menu);
    void buildUiSizeMenu();
    std::function<void(CommandRegistry&)> registerFrameCommands_;
    Runtime runtime_;
    CommandRegistry* commands_ = nullptr;
    DocumentRegistry* documents_ = nullptr;
    PanelHost* panels_ = nullptr;
    ImportSaveCoordinator* importSave_ = nullptr;
    ToolController* tools_ = nullptr;
    QMenuBar* menuBar_ = nullptr;
    QPointer<QMenu> documentMenu_;
    QPointer<ColorRangeDialog> colorRangeDialog_;
    QPointer<ReplaceColorDialog> replaceColorDialog_;
    QList<QAction*> recentActions_;
    bool clipboardMirrored_ = false;
    bool clipboardExported_ = false;
    bool clipboardSetting_ = false;
    void exportClipboard();
};

} // namespace pictura
