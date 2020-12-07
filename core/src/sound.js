export async function load(path) {
    const ctx = new AudioContext();
    const data = await fetch(path).then((res) => res.arrayBuffer());
    const audio = await ctx.decodeAudioData(data);
    return {
        _play: (volume) => {
            const gain = ctx.createGain();
            gain.gain.value = volume;
            gain.connect(ctx.destination);

            const source = ctx.createBufferSource();
            source.buffer = audio;
            source.connect(gain);

            source.start();
        },
    };
}
