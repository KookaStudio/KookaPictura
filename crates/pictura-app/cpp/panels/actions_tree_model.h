#pragma once

#include <QtCore/QAbstractItemModel>
#include <QtCore/QHash>
#include <QtCore/QJsonObject>
#include <QtCore/QList>
#include <QtCore/QSet>
#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QColor>
#include <QtGui/QIcon>
#include <QtWidgets/QTreeView>

#include <functional>
#include <memory>
#include <vector>

namespace pictura {

struct ActionNodeRow {
    QString id;
    QString parentId;
    QString kind;
    QString name;
    bool enabled = true;
    bool modal = false;
    int color = 0;
    int functionKey = 0;
    bool functionShift = false;
    bool functionCtrl = false;
    int stepIndex = -1;
    bool canExpand = false;
    int dialogOptions = 0;
    QString stepKind;
    QString commandId;
    QString message;
    bool allowContinue = false;
    QString condition;
    QString thenActionId;
    QString elseActionId;
    QString path;
    QJsonObject params;
};

inline QString actionKindSet() { return QStringLiteral("set"); }
inline QString actionKindAction() { return QStringLiteral("action"); }
inline QString actionKindStep() { return QStringLiteral("step"); }
inline QString actionKindParam() { return QStringLiteral("param"); }

enum ActionDataRole {
    ActionNodeIdRole = Qt::UserRole + 1,
    ActionNodeKindRole,
    ActionParentIdRole,
    ActionIncludedStateRole,
    ActionModalStateRole,
    ActionModalEditableRole,
    ActionInclusionEditableRole,
    ActionFunctionKeyRole,
    ActionStepCommandRole,
    ActionStepParamsRole,
    ActionColorRole,
};

QColor actionColor(int color);
QString actionColorName(int color);
QStringList actionColorNames();
QIcon actionCheckIcon(int state);
QIcon actionModalIcon(int state);
QString actionFunctionKeyLabel(int key, bool shift, bool ctrl);

class ActionsEngineApi : public QObject {
    Q_OBJECT

public:
    explicit ActionsEngineApi(QObject* parent = nullptr);
    ~ActionsEngineApi() override;

    virtual QList<ActionNodeRow> snapshot() const = 0;
    virtual QString lastError() const = 0;

    virtual QString createSet(const QString& name) = 0;
    virtual QString createAction(const QString& setId, const QString& name, int functionKey,
                                 bool shift, bool ctrl, int color) = 0;
    virtual bool renameNode(const QString& id, const QString& name) = 0;
    virtual bool deleteNodes(const QStringList& ids) = 0;
    virtual bool duplicateNodes(const QStringList& ids) = 0;
    virtual bool moveNode(const QString& id, const QString& parentId, int row, bool copy) = 0;
    virtual bool setIncluded(const QString& id, bool included) = 0;
    virtual bool toggleSiblingInclusion(const QString& id) = 0;
    virtual bool setModal(const QString& id, bool modal) = 0;
    virtual bool setActionOptions(const QString& id, const QString& name,
                                  const QString& targetSetId, int functionKey, bool shift,
                                  bool ctrl, int color) = 0;
    virtual bool setSetName(const QString& id, const QString& name) = 0;
    virtual bool assignFunctionKey(const QString& actionId, int functionKey, bool shift,
                                   bool ctrl, QString* error) = 0;
    virtual bool editStep(const QString& stepId, const QJsonObject& params) = 0;

    virtual bool startRecording(const QString& actionId, int stepIndex) = 0;
    virtual void stopRecording() = 0;
    virtual bool isRecording() const = 0;
    virtual bool recordAgain(const QString& actionId) = 0;
    virtual QString recordAgainNext(QJsonObject* params) = 0;
    virtual bool recordToolStroke(const QString& toolId, const QJsonObject& params,
                                  int payloadBytes) = 0;
    virtual void recordCommand(const QString& commandId, const QJsonObject& params) = 0;
    virtual void setAllowToolRecording(bool on) = 0;
    virtual bool allowToolRecording() const = 0;

    virtual bool startPlayback(const QStringList& stepIds) = 0;
    virtual void setDocumentFacts(bool present, bool square, const QString& mode,
                                  bool hasSelection) = 0;
    virtual ActionNodeRow playbackNext() = 0;
    virtual void playbackContinue() = 0;
    virtual void cancelPlayback() = 0;
    virtual bool isPlaying() const = 0;
    virtual QString conditionalBranch(const QString& stepId) = 0;
    virtual bool applyPathStep(const QString& stepId) = 0;

    virtual bool createStopStep(const QString& anchorId, const QString& message,
                                bool allowContinue) = 0;
    virtual bool createMenuItemStep(const QString& anchorId, const QString& commandId) = 0;
    virtual bool createPathStep(const QString& anchorId, const QString& path) = 0;
    virtual bool createConditionalStep(const QString& anchorId, const QString& condition,
                                       const QString& thenActionId,
                                       const QString& elseActionId, QString* error) = 0;
    virtual QStringList actionIdsInSet(const QString& setId) const = 0;

    virtual bool loadSetFile(const QString& path, bool replace) = 0;
    virtual bool saveSetFile(const QString& setId, const QString& path) = 0;
    virtual bool writeTextDump(const QString& setId, const QString& path) = 0;
    virtual bool loadLibrary() = 0;
    virtual bool saveLibrary() = 0;
    virtual bool reset(bool append) = 0;
    virtual bool clearAll() = 0;

    virtual int playbackMode() const = 0;
    virtual double playbackPauseSeconds() const = 0;
    virtual void setPlaybackOptions(int mode, double pauseSeconds) = 0;
    virtual void setWindowsPlatform(bool windows) = 0;

signals:
    void changed();
    void recordingChanged(bool recording);
    void playbackChanged(bool playing);
    void error(const QString& message);
};

class ActionsTreeModel : public QAbstractItemModel {
    Q_OBJECT

public:
    explicit ActionsTreeModel(QObject* parent = nullptr);
    ~ActionsTreeModel() override;

    void setRows(const QList<ActionNodeRow>& rows);

    QModelIndex index(int row, int column, const QModelIndex& parent = QModelIndex()) const override;
    QModelIndex parent(const QModelIndex& child) const override;
    int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    int columnCount(const QModelIndex& parent = QModelIndex()) const override;
    QVariant data(const QModelIndex& index, int role = Qt::DisplayRole) const override;
    bool setData(const QModelIndex& index, const QVariant& value,
                 int role = Qt::EditRole) override;
    QVariant headerData(int section, Qt::Orientation orientation,
                        int role = Qt::DisplayRole) const override;
    Qt::ItemFlags flags(const QModelIndex& index) const override;

    QStringList mimeTypes() const override;
    QMimeData* mimeData(const QModelIndexList& indexes) const override;
    bool canDropMimeData(const QMimeData* data, Qt::DropAction action, int row, int column,
                         const QModelIndex& parent) const override;
    bool dropMimeData(const QMimeData* data, Qt::DropAction action, int row, int column,
                      const QModelIndex& parent) override;
    Qt::DropActions supportedDropActions() const override;
    Qt::DropActions supportedDragActions() const override;

    QString nodeId(const QModelIndex& index) const;
    QString nodeKind(const QString& id) const;
    ActionNodeRow rowForId(const QString& id) const;
    QModelIndex indexForId(const QString& id) const;
    QStringList actionIdsForSet(const QString& setId) const;
    QStringList stepIdsForAction(const QString& actionId) const;

    QString identityForId(const QString& id) const;
    QString idForIdentity(const QString& identity) const;
    QStringList identitiesForIds(const QStringList& ids) const;
    QStringList idsForIdentities(const QStringList& identities) const;

    int inclusionState(const QModelIndex& index) const;
    int modalState(const QModelIndex& index) const;
    bool modalEditable(const QModelIndex& index) const;

    void requestInclusion(const QModelIndex& index, bool included);
    void toggleSiblingsIncluded(const QModelIndex& index);
    void requestModal(const QModelIndex& index, bool modal);

signals:
    void inclusionRequested(const QString& id, bool included);
    void siblingInclusionRequested(const QString& id);
    void modalRequested(const QString& id, bool modal);
    void renameRequested(const QString& id, const QString& name);
    void moveRequested(const QString& id, const QString& parentId, int row, bool copy);

private:
    struct Node {
        ActionNodeRow row;
        Node* parent = nullptr;
        QList<Node*> children;
        int rowInParent = 0;
        bool param = false;
    };

    Node* nodeFor(const QModelIndex& index) const;
    Node* nodeForId(const QString& id) const;
    QModelIndex indexForNode(const Node* node, int column = 0) const;
    int stateFor(const Node* node) const;
    int modalStateFor(const Node* node) const;
    bool modalEditableFor(const Node* node) const;
    bool nodeEditableName(const Node* node) const;
    void notifyAncestors(Node* node);
    void setIncludedLocally(Node* node, bool included);
    void setModalLocally(Node* node, bool modal);
    QStringList selectedIdsFromMime(const QMimeData* data) const;
    static QString encodeIdentity(const QList<QPair<QString, int>>& segments);

    std::vector<std::unique_ptr<Node>> nodes_;
    QList<Node*> roots_;
    QHash<QString, Node*> byId_;
};

class ActionsTreeView : public QTreeView {
    Q_OBJECT

public:
    explicit ActionsTreeView(QWidget* parent = nullptr);

signals:
    void inclusionToggleRequested(const QModelIndex& index);
    void siblingInclusionToggleRequested(const QModelIndex& index);
    void modalToggleRequested(const QModelIndex& index);
    void escapePressed();

protected:
    void mousePressEvent(QMouseEvent* event) override;
    void keyPressEvent(QKeyEvent* event) override;

private:
    bool atDisclosure(const QModelIndex& index, const QPoint& pos) const;
};

} // namespace pictura
