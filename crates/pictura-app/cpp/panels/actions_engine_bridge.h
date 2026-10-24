#pragma once

#include "actions_tree_model.h"

namespace pictura {

class ActionsEngine;

class ActionsEngineBridge : public ActionsEngineApi {
public:
    explicit ActionsEngineBridge(ActionsEngine* engine, QObject* parent = nullptr);
    ~ActionsEngineBridge() override;

    QList<ActionNodeRow> snapshot() const override;
    QString lastError() const override;

    QString createSet(const QString& name) override;
    QString createAction(const QString& setId, const QString& name, int functionKey, bool shift,
                         bool ctrl, int color) override;
    bool renameNode(const QString& id, const QString& name) override;
    bool deleteNodes(const QStringList& ids) override;
    bool duplicateNodes(const QStringList& ids) override;
    bool moveNode(const QString& id, const QString& parentId, int row, bool copy) override;
    bool setIncluded(const QString& id, bool included) override;
    bool toggleSiblingInclusion(const QString& id) override;
    bool setModal(const QString& id, bool modal) override;
    bool setActionOptions(const QString& id, const QString& name, const QString& targetSetId,
                          int functionKey, bool shift, bool ctrl, int color) override;
    bool setSetName(const QString& id, const QString& name) override;
    bool assignFunctionKey(const QString& actionId, int functionKey, bool shift, bool ctrl,
                           QString* error) override;
    bool editStep(const QString& stepId, const QJsonObject& params) override;

    bool startRecording(const QString& actionId, int stepIndex) override;
    void stopRecording() override;
    bool isRecording() const override;
    bool recordAgain(const QString& actionId) override;
    QString recordAgainNext(QJsonObject* params) override;
    bool recordToolStroke(const QString& toolId, const QJsonObject& params,
                          int payloadBytes) override;
    bool recordToolStrokePayload(const QString& payload);
    void toolStrokeRefused(const QString& message);
    void recordCommand(const QString& commandId, const QJsonObject& params) override;
    void setAllowToolRecording(bool on) override;
    bool allowToolRecording() const override;

    bool startPlayback(const QStringList& stepIds) override;
    void setDocumentFacts(bool present, bool square, const QString& mode,
                          bool hasSelection) override;
    ActionNodeRow playbackNext() override;
    void playbackContinue() override;
    void cancelPlayback() override;
    bool isPlaying() const override;
    QString conditionalBranch(const QString& stepId) override;
    bool applyPathStep(const QString& stepId) override;

    bool createStopStep(const QString& anchorId, const QString& message,
                        bool allowContinue) override;
    bool createMenuItemStep(const QString& anchorId, const QString& commandId) override;
    bool createPathStep(const QString& anchorId, const QString& path) override;
    bool createConditionalStep(const QString& anchorId, const QString& condition,
                               const QString& thenActionId, const QString& elseActionId,
                               QString* error) override;
    QStringList actionIdsInSet(const QString& setId) const override;

    bool loadSetFile(const QString& path, bool replace) override;
    bool saveSetFile(const QString& setId, const QString& path) override;
    bool writeTextDump(const QString& setId, const QString& path) override;
    bool loadLibrary() override;
    bool saveLibrary() override;
    bool reset(bool append) override;
    bool clearAll() override;

    int playbackMode() const override;
    double playbackPauseSeconds() const override;
    void setPlaybackOptions(int mode, double pauseSeconds) override;
    void setWindowsPlatform(bool windows) override;

private:
    struct Ref {
        int set = -1;
        int action = -1;
        int step = -1;
    };

    static Ref parseId(const QString& id);
    static QString makeId(const Ref& ref);
    static int keyNumber(int key);
    void refreshSignals();
    QString actionNameFor(const QString& idOrName) const;
    QString palettePath() const;

    Ref recordAgainAction_;
    int recordAgainStep_ = 0;
    bool recordingState_ = false;
    bool playbackState_ = false;
    QString refusedError_;
    QString emittedError_;
    QString lastEngineError_;
    ActionsEngine* engine_ = nullptr;
};

// The paint-tool recorder seam: the frame installs its live bridge here so the
// Qt-free stroke capture can append a step without a Rust pointer back into Qt.
void set_active_actions_engine(ActionsEngineBridge* bridge);

} // namespace pictura
