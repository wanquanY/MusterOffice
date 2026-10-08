/** Types of the audited source seam; the source tree/ABI is pinned by builder. */
declare module '@musteroffice/mediabunny-aac' {
  export function parseAacAudioSpecificConfig(bytes: Uint8Array): {
    objectType: number; coreObjectType: number; frequencyIndex: number; channelConfiguration: number;
    outputSampleRate: number | null; outputNumberOfChannels: number | null;
  };
  export const aacFrequencyTable: number[];
}
