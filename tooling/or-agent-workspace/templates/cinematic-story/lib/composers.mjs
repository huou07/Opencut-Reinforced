// This film's composer map. Each key is one beat in video.config.json.
import * as open from './scenes/open.mjs';
import * as edit from './scenes/edit.mjs';
import * as assistant from './scenes/assistant.mjs';
import * as motion from './scenes/motion.mjs';
import * as local from './scenes/local.mjs';
import * as value from './scenes/value.mjs';
import * as community from './scenes/community.mjs';
import * as extra from './scenes/extra.mjs';

export const COMPOSERS = {
  coldOpen: open.coldOpen,
  layersAlign: extra.layersAlign,
  cutRipple: extra.cutRipple,
  assistantRole: extra.assistantRole,
  harnessRail: extra.harnessRail,
  mitDoc: extra.mitDoc,
  localCost: extra.localCost,
  releaseCard: extra.releaseCard,
  providerTrap: extra.providerTrap,
  checks: extra.checks,
  layerFlow: open.layerFlow,
  reveal: open.reveal,
  editorBuild: edit.editorBuild,
  editActions: edit.editActions,
  previewExport: edit.previewExport,
  assistantScope: assistant.assistantScope,
  chatAsk: assistant.chatAsk,
  acpBridge: assistant.acpBridge,
  chatResult: assistant.chatResult,
  motionCode: motion.motionCode,
  motionBuild: motion.motionBuild,
  motionMorph: motion.motionMorph,
  motionTimeline: motion.motionTimeline,
  ttsScript: local.ttsScript,
  ttsWave: local.ttsWave,
  whisperListen: local.whisperListen,
  whisperCues: local.whisperCues,
  workflowLoop: local.workflowLoop,
  freeLicense: value.freeLicense,
  ownership: value.ownership,
  reality: community.reality,
  platformGap: community.platformGap,
  mobileQuestion: community.mobileQuestion,
  notarize: community.notarize,
  community: community.community,
  contribPath: community.contribPath,
  commitment: community.commitment,
  close: community.close,
};

export const COMPOSER_NAMES = Object.keys(COMPOSERS);
