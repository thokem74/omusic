// Throwaway UX simulation. No filesystem access, backend, persistence, or audio.
const albums = [
  { id: 'night', title: 'Night Signals', artist: 'Aster Vale', year: '2023', genre: 'Electronic / ambient', edition: 'Original', tracks: [
    ['First Light', 234], ['Glasshouse', 287], ['Slow Satellites', 312], ['Open Circuit', 246, 'Aster Vale feat. Mira Sol', ['Aster Vale', 'Mira Sol']], ['Night Signals', 365], ['Home Before Dawn', 261]] },
  { id: 'blue', title: 'The Blue Hour', artist: 'Aster Vale', year: '2021', genre: 'Ambient', edition: 'Original', tracks: [
    ['Soft Focus', 278], ['Underwater Cities', 341], ['The Blue Hour', 298], ['A Place to Land', 260], ['Afterimage', 319]] },
  { id: 'interval', title: 'Intervals', artist: 'Low Orbit', year: '2024', genre: 'Downtempo', edition: 'Original', tracks: [
    ['Passing Through', 242], ['Halfway Home', 314], ['Quiet Machines', 269], ['Long Way Round', 291], ['Between Stations', 337], ['Last Train', 223]] },
  { id: 'water', title: 'Still Water', artist: 'Mira Sol', year: '2022', genre: 'Soul / electronic', edition: 'Original', tracks: [
    ['Riverside', 217], ['Weightless', 254], ['Still Water', 302], ['The Other Side', 238], ['Soft Landing', 276]] },
  { id: 'after', title: 'After Hours, Vol. 2', artist: 'Various Artists', year: '2024', genre: 'Compilation / electronic', edition: 'Original', tracks: [
    ['City Without Sleep', 263, 'Low Orbit'], ['Warm Windows', 248, 'Mira Sol'], ['Side Streets', 296, 'Aster Vale'], ['Room for Two', 231, 'Paper Planes'], ['Faint Transmission', 321, 'Low Orbit'], ['Morning, Eventually', 278, 'Mira Sol']] },
  { id: 'night-remaster', title: 'Night Signals', artist: 'Aster Vale', year: '2025', genre: 'Electronic / ambient', edition: 'Remaster', tracks: [
    ['First Light', 235], ['Glasshouse', 288], ['Slow Satellites', 312], ['Open Circuit', 247, 'Aster Vale feat. Mira Sol', ['Aster Vale', 'Mira Sol']], ['Night Signals', 366], ['Home Before Dawn', 262]] },
];
albums.forEach(album => {
  album.tracks = album.tracks.map((t, i) => ({ id: `${album.id}-${i}`, albumId: album.id, index: i, title: t[0], duration: t[1], artist: t[2] || album.artist, contributors: t[3] || [t[2] || album.artist] }));
});
const artists = [...new Set(albums.flatMap(a => [a.artist, ...a.tracks.flatMap(t => t.contributors)]))].sort();
const allTracks = albums.flatMap(a => a.tracks);
const variants = {
  A: { name: 'Three columns', title: 'A / The library', section: 'LIBRARY', description: 'Everything stays in view. Move from artist to album to track without losing your place. The familiar, information-dense option.', task: 'Choose Mira Sol, then open Night Signals under Appears on. Her contribution is highlighted inside the full album.' },
  B: { name: 'Album shelf', title: 'B / The record shelf', section: 'ALBUMS', description: 'Albums lead the experience. Scan a shelf of releases, filter by artist, and open the track list alongside it. More room for each record.', task: 'Filter to Aster Vale. Compare the original Night Signals with its remaster, then add an album to the queue.' },
  C: { name: 'Search first', title: 'C / Find your next track', section: 'DISCOVER', description: 'A command-palette approach. Start with a few letters, jump to an artist, album, or track, and keep the listening context nearby.', task: 'Search “Mira” to find her albums and contributions. Open a track result to play from that track within its complete album.' },
  D: { name: 'Queue desk', title: 'D / The listening desk', section: 'QUEUE', description: 'The listening sequence is the main workspace. Browse at the left, build your queue in the middle, and keep the current track at the right.', task: 'Add another album, move an entry up or down, and remove the current track. Watch the queue and playback state change.' },
  E: { name: 'Single pane', title: 'E / One thing at a time', section: 'BROWSE', description: 'One focused list, with a breadcrumb back home. Drill from artist to album to tracks. A calmer option for smaller terminal windows.', task: 'Open an artist, then an album. Use Backspace to step back. Play an album and browse elsewhere while the playback strip stays put.' },
};
const initialVariant = new URLSearchParams(location.search).get('variant');
const s = { variant: variants[initialVariant] ? initialVariant : 'A', artist: 'Aster Vale', album: 'night', selectedTrack: 1,
  depth: 'artists', query: '', searchOpen: false, queueOpen: false, searchContext: 'album', shelfFilter: 'All', queue: [...albums[0].tracks], current: 1,
  playing: false, elapsed: 72, volume: 70, shuffle: false, repeat: 'off', shuffleRemaining: [], history: [],
  notice: 'Sample data loaded. Nothing touches your music files.' };
const $ = id => document.getElementById(id);
const escapeHtml = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const e = escapeHtml;
const time = seconds => `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, '0')}`;
const albumById = id => albums.find(a => a.id === id);
const chosenAlbum = () => albumById(s.album);
const currentTrack = () => s.queue[s.current];
const label = album => `${album.title}${album.edition === 'Original' ? '' : ' · ' + album.edition}`;
const ownAlbums = () => albums.filter(a => a.artist === s.artist);
const appearsOn = () => albums.filter(a => a.artist !== s.artist && a.tracks.some(t => t.contributors.includes(s.artist)));
const countFor = artist => albums.filter(a => a.artist === artist || a.tracks.some(t => t.contributors.includes(artist))).length;
function action(text, act, extra = '', primary = false) { return `<button class="action ${primary ? 'primary' : ''}" data-act="${act}" ${extra}>${text}</button>`; }
function albumInfo(album, buttons = true) {
  return `<div class="album-meta"><div>${e(album.artist)}</div><h2>${e(album.title)}</h2><div class="meta-line">${e(album.year)} · ${album.tracks.length} tracks · ${time(album.tracks.reduce((n,t) => n + t.duration, 0))}</div><div class="meta-line">${e(album.genre)}${album.edition !== 'Original' ? ' · ' + e(album.edition) : ''}</div>${buttons ? `<div class="actions">${action('▶ Play album', 'play-album', '', true)}${action('+ Add to queue', 'add-album')}</div>` : ''}</div>`;
}
function artistRow(artist, prefix = '') {
  return `<button id="${prefix}artist-${artists.indexOf(artist)}" class="row ${s.artist === artist ? 'selected' : ''}" data-nav data-act="artist" data-artist="${e(artist)}"><span class="main-text">${e(artist)}</span><span class="count">${countFor(artist)}</span></button>`;
}
function albumRow(album, appearance = false, prefix = '') {
  return `<button id="${prefix}album-${album.id}" class="row ${s.album === album.id ? 'selected' : ''}" data-nav data-act="album" data-album="${album.id}"><span class="main-text">${e(label(album))}<span class="credit">${e(appearance ? album.artist : album.year)}</span></span>${appearance ? '<span class="pill">appears on</span>' : `<span class="count">${album.tracks.length}</span>`}</button>`;
}
function trackRow(track, prefix = '') {
  const album = albumById(track.albumId);
  const now = currentTrack()?.id === track.id;
  const contributes = album.artist !== s.artist && track.contributors.includes(s.artist);
  return `<button id="${prefix}track-${track.id}" class="row ${s.album === track.albumId && s.selectedTrack === track.index ? 'selected' : ''} ${now ? 'playing' : ''}" data-nav data-act="track" data-album="${album.id}" data-index="${track.index}"><span class="number">${now ? (s.playing ? '▶' : 'Ⅱ') : String(track.index + 1).padStart(2, '0')}</span><span class="main-text">${e(track.title)}${track.artist !== album.artist ? `<span class="credit">${e(track.artist)}</span>` : ''}</span>${contributes ? '<span class="highlight" title="This artist contributes to this track">◆</span>' : ''}<span class="duration">${time(track.duration)}</span></button>`;
}
function artistAlbums() {
  const own = ownAlbums(), appearances = appearsOn();
  return `${own.length ? own.map(a => albumRow(a)).join('') : '<div class="empty">No albums as album artist.</div>'}${appearances.length ? `<div class="section-caption">Appears on · ${appearances.length}</div>${appearances.map(a => albumRow(a, true)).join('')}` : ''}`;
}
function VariantA() {
  return `<div class="columns"><section class="pane" data-list><div class="pane-head">1 Artists <small>${artists.length}</small></div>${artists.map(a => artistRow(a)).join('')}</section><section class="pane" data-list><div class="pane-head">2 Albums <small>${e(s.artist)}</small></div>${artistAlbums()}</section><section class="pane" data-list><div class="pane-head">3 Tracks <small>Enter to play</small></div>${albumInfo(chosenAlbum())}${chosenAlbum().tracks.map(t => trackRow(t)).join('')}</section></div>`;
}
function VariantB() {
  const shown = s.shelfFilter === 'All' ? albums : albums.filter(a => a.artist === s.shelfFilter || a.tracks.some(t => t.contributors.includes(s.shelfFilter)));
  return `<div class="shelf-layout"><section class="shelf" data-list><div class="pane-head">The collection <small>${shown.length} albums</small></div><div class="artist-filter">${['All', ...artists].map(a => `<button class="filter ${s.shelfFilter === a ? 'active' : ''}" data-act="filter" data-artist="${e(a)}">${e(a)}</button>`).join('')}</div>${shown.map((a,i) => `<button id="album-${a.id}" class="album-tile ${s.album === a.id ? 'selected' : ''}" data-nav data-act="album" data-album="${a.id}"><span class="album-index">${String(i+1).padStart(2,'0')}</span><span><span class="album-name">${e(a.title)}</span><small>${e(a.artist)}${a.edition !== 'Original' ? ' / '+e(a.edition) : ''}${s.shelfFilter !== 'All' && a.artist !== s.shelfFilter ? ' / appears on' : ''}</small></span><span class="album-year">${a.year}<br>${a.tracks.length} tracks</span></button>`).join('')}</section><section class="shelf-detail" data-list>${albumInfo(chosenAlbum())}${chosenAlbum().tracks.map(t => trackRow(t)).join('')}</section></div>`;
}
function searchResults(prefix = 'result-') {
  const query = s.query.toLocaleLowerCase().trim();
  const match = text => text.toLocaleLowerCase().includes(query);
  const matchingArtists = artists.filter(match);
  const matchingAlbums = albums.filter(a => match(`${a.title} ${a.artist} ${a.edition}`));
  const matchingTracks = allTracks.filter(t => match(`${t.title} ${t.artist} ${albumById(t.albumId).title}`));
  if (!matchingArtists.length && !matchingAlbums.length && !matchingTracks.length) return '<div class="empty">No matches. Try an artist, album, or track title.</div>';
  return `${matchingArtists.length ? '<div class="section-caption">Artists</div>' : ''}${matchingArtists.map(a => `<button id="${prefix}artist-${artists.indexOf(a)}" class="row" data-nav data-act="search-artist" data-artist="${e(a)}"><span class="result-kind">ARTIST</span><span class="main-text">${e(a)}</span><span class="count">${countFor(a)} albums →</span></button>`).join('')}${matchingAlbums.length ? '<div class="section-caption">Albums</div>' : ''}${matchingAlbums.map(a => `<button id="${prefix}album-${a.id}" class="row" data-nav data-act="search-album" data-album="${a.id}"><span class="result-kind">ALBUM</span><span class="main-text">${e(label(a))}<span class="credit">${e(a.artist)}</span></span><span class="count">${a.year} →</span></button>`).join('')}${matchingTracks.length ? '<div class="section-caption">Tracks · Enter plays within the album</div>' : ''}${matchingTracks.map(t => `<button id="${prefix}track-${t.id}" class="row" data-nav data-act="search-track" data-album="${t.albumId}" data-index="${t.index}"><span class="result-kind">TRACK</span><span class="main-text">${e(t.title)}<span class="credit">${e(t.artist)} · ${e(label(albumById(t.albumId)))}</span></span><span class="duration">${time(t.duration)}</span></button>`).join('')}`;
}
function searchInput(id = 'search', large = true) {
  return `<div class="${large ? 'large-search' : 'mini-search'}"><span>/</span><input id="${id}" aria-label="Search collection" autocomplete="off" placeholder="Artist, album, or track…" value="${e(s.query)}"><button class="search-close" data-act="${large ? 'clear-search' : 'close-search'}">${large ? 'clear' : 'Esc ×'}</button></div>`;
}
function VariantC() {
  const context = s.searchContext === 'artist' ? `<h3>Artist context</h3><div class="album-meta"><h2>${e(s.artist)}</h2></div>${artistAlbums()}` : `<h3>Album context</h3>${albumInfo(chosenAlbum())}<div class="section-caption">Tracks</div>${chosenAlbum().tracks.map(t => trackRow(t, 'context-')).join('')}`;
  return `<div class="search-layout"><section class="search-main" data-list><h2 class="search-heading">What do you want to hear?</h2><p class="search-subheading">Search the whole collection. Every album stays intact.</p>${searchInput()}${searchResults()}</section><aside class="search-side" data-list>${context}</aside></div>`;
}
function queueRows() {
  return s.queue.length ? s.queue.map((t,i) => `<div class="queue-row ${i === s.current ? 'current' : ''}"><span class="queue-order">${i === s.current ? (s.playing ? '▶' : 'Ⅱ') : String(i + 1).padStart(2,'0')}</span><button id="queue-${i}" data-nav data-act="play-queue" data-index="${i}" style="text-align:left;min-width:0"><div class="queue-title">${e(t.title)}</div><div class="queue-credit">${e(t.artist)}</div></button><span><button data-act="move-up" data-index="${i}" aria-label="Move ${e(t.title)} up">↑</button><button data-act="move-down" data-index="${i}" aria-label="Move ${e(t.title)} down">↓</button><button data-act="remove" data-index="${i}" aria-label="Remove ${e(t.title)}">×</button></span></div>`).join('') : '<div class="empty">The queue is empty.<br>Add an album from the library.</div>';
}
function VariantD() {
  const now = currentTrack(), album = now && albumById(now.albumId);
  return `<div class="queue-desk"><section class="pane" data-list><div class="pane-head">Library <small>choose & add</small></div>${albums.map(a => albumRow(a)).join('')}<div class="actions">${action('+ Add selected album', 'add-album')}</div></section><section class="pane" data-list><div class="pane-head">Up next <small>${s.queue.length} tracks</small></div>${queueRows()}<div class="queue-tools"><button data-act="clear-queue">× Clear queue</button><span>${s.shuffle ? 'Shuffle on · order preserved' : 'Sequential playback'}</span></div></section><aside class="pane"><div class="pane-head">Now playing</div><div class="now-credit">${s.playing ? '● PLAYING' : 'Ⅱ PAUSED'}</div><div class="now-title">${e(now?.title || 'Nothing queued')}</div><div class="now-credit">${e(now?.artist || 'Add an album to begin')}</div><div class="now-album">${e(album ? label(album) : '')}</div><div class="inspector-rule"></div><div class="inspector-label">SELECTED ALBUM</div><div class="inspector-value">${e(label(chosenAlbum()))}</div><div class="now-album">${e(chosenAlbum().artist)}</div><div class="actions">${action('▶ Replace & play', 'play-album')}</div><div class="inspector-label">SESSION</div><div class="now-album">Close the TUI → stop playback<br>Reopen → restore, stay stopped</div></aside></div>`;
}
function VariantE() {
  let content;
  const crumbs = `<div class="breadcrumbs"><button data-act="depth" data-depth="artists">Collection</button>${s.depth !== 'artists' ? `<span>/</span><button data-act="depth" data-depth="albums">${e(s.artist)}</button>` : ''}${s.depth === 'tracks' ? `<span>/</span><span>${e(label(chosenAlbum()))}</span>` : ''}</div>`;
  if (s.depth === 'artists') content = `<h2>Your collection</h2><div class="single-sub">${artists.length} artists · ${albums.length} albums · ${allTracks.length} tracks</div><div data-list>${artists.map(a => artistRow(a)).join('')}</div>`;
  if (s.depth === 'albums') content = `<h2>${e(s.artist)}</h2><div class="single-sub">Albums & contributions · Backspace to go back</div><div data-list>${artistAlbums()}</div>`;
  if (s.depth === 'tracks') content = `${albumInfo(chosenAlbum())}<div data-list>${chosenAlbum().tracks.map(t => trackRow(t)).join('')}</div>`;
  return `<div class="single">${crumbs}${content}</div>`;
}
function renderPlayer() {
  const track = currentTrack();
  const album = track && albumById(track.albumId);
  return `<div class="player"><div><div class="playing-track"><span>${s.playing ? '▶' : 'Ⅱ'}</span>${e(track?.title || 'Queue empty')}</div><div class="playing-credit">${e(track ? `${track.artist} / ${label(album)}` : 'Choose an album to begin')}</div></div><div class="controls"><div class="transport"><button data-act="previous" aria-label="Previous track">Ⅰ◀</button><button class="play" data-act="toggle-play" aria-label="${s.playing ? 'Pause' : 'Play'}">${s.playing ? 'Ⅱ' : '▶'}</button><button data-act="next" aria-label="Next track">▶Ⅰ</button></div><div class="seek"><span>${time(s.elapsed)}</span><input id="seek" aria-label="Seek position" type="range" min="0" max="${track?.duration || 1}" value="${s.elapsed}"><span>${time(track?.duration || 0)}</span></div></div><div class="modes"><button class="${s.shuffle ? 'on' : ''}" data-act="shuffle">${s.shuffle ? '↝ on' : '↝ shuffle'}</button><button class="${s.repeat !== 'off' ? 'on' : ''}" data-act="repeat">↻ ${s.repeat}</button><label title="Volume"><input id="volume" class="volume" aria-label="Volume" type="range" min="0" max="100" value="${s.volume}"><span>${s.volume}%</span></label></div></div>`;
}
function render() {
  const focused = document.activeElement?.id;
  const cursor = document.activeElement?.selectionStart;
  $('variant-title').textContent = variants[s.variant].title;
  $('variant-description').textContent = variants[s.variant].description;
  $('try-this').textContent = variants[s.variant].task;
  $('section-label').textContent = variants[s.variant].section;
  document.querySelector('.collection-count').textContent = `${artists.length} artists / ${albums.length} albums / ${allTracks.length} tracks`;
  const views = { A: VariantA, B: VariantB, C: VariantC, D: VariantD, E: VariantE };
  $('screen').innerHTML = s.queueOpen ? `<div class="single" data-list><div class="single-actions"><h2>Playback queue</h2>${action('← Back to browser', 'show-queue')}</div>${queueRows()}<div class="queue-tools"><button data-act="clear-queue">× Clear queue</button><span>${s.queue.length} entries · ${s.shuffle ? 'shuffle on' : 'sequential'}</span></div></div>` : s.searchOpen && s.variant !== 'C' ? `<div class="search-main" data-list style="height:100%">${searchInput('search', false)}${searchResults('overlay-')}</div>` : views[s.variant]();
  $('player').innerHTML = renderPlayer();
  $('keybar').innerHTML = `<div class="keybar"><button data-act="help"><kbd>?</kbd>help</button><button data-act="open-search"><kbd>/</kbd>search</button><span><kbd>j k / ↑ ↓</kbd>navigate</span><span><kbd>Enter</kbd>open / play</span><button data-act="toggle-play"><kbd>Space</kbd>play / pause</button><button data-act="add-album"><kbd>a</kbd>add album</button>${action('Queue '+s.queue.length, 'show-queue')}</div>`;
  $('notice').textContent = s.notice;
  $('variant-buttons').innerHTML = Object.entries(variants).map(([key,v]) => `<button class="variant-choice ${s.variant === key ? 'active' : ''}" aria-pressed="${s.variant === key}" data-act="variant" data-variant="${key}"><span class="letter">${key}</span><span>${e(v.name)}</span></button>`).join('');
  $('state').textContent = JSON.stringify({ variant: s.variant, artist: s.artist, album: label(chosenAlbum()), selectedTrack: chosenAlbum().tracks[s.selectedTrack]?.title, singlePaneLevel: s.depth, search: s.query, queue: s.queue.map(t => `${t.artist} — ${t.title}`), currentQueueIndex: s.current, playing: s.playing, elapsedSeconds: s.elapsed, volume: s.volume, shuffle: s.shuffle, repeat: s.repeat, storage: 'memory only', audio: 'simulated' }, null, 2);
  if (focused && $(focused)) {
    $(focused).focus({ preventScroll: true });
    if (typeof cursor === 'number' && $(focused).type === 'text') $(focused).setSelectionRange(cursor, cursor);
  }
  const cols = Math.floor($('screen').clientWidth / 8.4);
  $('dimensions').textContent = `≈ ${cols} columns · browser simulation`;
}
function notice(text) { s.notice = text; }
function selectArtist(artist) {
  s.artist = artist;
  s.searchContext = 'artist';
  const album = ownAlbums()[0] || appearsOn()[0];
  if (album) { s.album = album.id; s.selectedTrack = 0; }
  s.depth = 'albums';
  notice(`Browsing ${artist}. Albums and contributions remain complete.`);
}
function selectAlbum(id) {
  s.album = id; s.selectedTrack = 0; s.depth = 'tracks';
  s.searchContext = 'album';
  notice(`Opened ${label(chosenAlbum())}. Enter on a track to play; a adds the album.`);
}
function resetShuffle() {
  s.shuffleRemaining = s.queue.map((_, i) => i).filter(i => i !== s.current);
}
function playAlbum(id = s.album, index = 0) {
  s.album = id; s.selectedTrack = index; s.queue = [...albumById(id).tracks]; s.current = index;
  s.playing = true; s.elapsed = 0; s.history = []; resetShuffle();
  notice(`Simulated playback: ${s.queue[index].title}. Queue replaced with the full album.`);
}
function nextTrack(automatic = false) {
  if (!s.queue.length) return;
  s.history.push(s.current);
  if (automatic && s.repeat === 'track') { s.elapsed = 0; return; }
  if (s.shuffle) {
    if (!s.shuffleRemaining.length && s.repeat === 'queue') resetShuffle();
    if (s.shuffleRemaining.length) {
      const at = Math.floor(Math.random() * s.shuffleRemaining.length);
      s.current = s.shuffleRemaining.splice(at, 1)[0];
    } else { s.playing = false; s.elapsed = 0; notice('End of shuffled queue.'); return; }
  } else if (s.current < s.queue.length - 1) s.current++;
  else if (s.repeat === 'queue') s.current = 0;
  else { s.playing = false; s.elapsed = 0; notice('End of queue.'); return; }
  s.elapsed = 0; notice(`Selected ${currentTrack().title}.`);
}
function switchVariant(key) {
  if (!variants[key]) return;
  s.variant = key; s.searchOpen = false; s.queueOpen = false;
  const url = new URL(location.href); url.searchParams.set('variant', key); history.replaceState({}, '', url);
  notice(`Variant ${key}: ${variants[key].name}. Shared browsing and playback state preserved.`);
  render();
}
function cycleVariant(direction) {
  const keys = Object.keys(variants); switchVariant(keys[(keys.indexOf(s.variant) + direction + keys.length) % keys.length]);
}
function handleAction(button) {
  const { act, artist, album, index, depth, variant } = button.dataset;
  const i = Number(index);
  switch (act) {
    case 'variant': switchVariant(variant); return;
    case 'artist': selectArtist(artist); break;
    case 'album': selectAlbum(album); break;
    case 'track': s.album = album; s.selectedTrack = i; notice(`${chosenAlbum().tracks[i].title} selected. Enter or double-click to play.`); break;
    case 'filter':
      s.shelfFilter = artist;
      if (artist !== 'All') selectArtist(artist);
      notice(`Album shelf filter: ${artist}. Contributions are included.`); break;
    case 'play-album': playAlbum(); break;
    case 'add-album': s.queue.push(...chosenAlbum().tracks); resetShuffle(); notice(`Added ${chosenAlbum().tracks.length} tracks from ${label(chosenAlbum())}.`); break;
    case 'toggle-play': if (s.queue.length) { s.playing = !s.playing; notice(s.playing ? 'Simulated playback started. No audio is produced.' : 'Paused.'); } else notice('Queue empty. Choose an album first.'); break;
    case 'next': nextTrack(); break;
    case 'previous': if (s.elapsed > 3) s.elapsed = 0; else { s.current = s.history.pop() ?? Math.max(0, s.current - 1); s.elapsed = 0; } notice('Previous / restart current track.'); break;
    case 'shuffle': s.shuffle = !s.shuffle; resetShuffle(); notice(`Shuffle ${s.shuffle ? 'on' : 'off'}. Visible queue order is unchanged.`); break;
    case 'repeat': s.repeat = ['off','track','queue'][(['off','track','queue'].indexOf(s.repeat)+1)%3]; notice(`Repeat: ${s.repeat}.`); break;
    case 'play-queue': s.current = i; s.elapsed = 0; s.playing = true; resetShuffle(); notice(`Simulated playback: ${currentTrack().title}.`); break;
    case 'clear-queue': s.queue = []; s.current = 0; s.elapsed = 0; s.playing = false; s.history = []; resetShuffle(); notice('Queue cleared. Playback stopped.'); break;
    case 'remove': {
      const wasCurrent = i === s.current;
      s.queue.splice(i, 1);
      if (i < s.current) s.current--;
      if (!s.queue.length || s.current >= s.queue.length) { s.playing = false; s.current = Math.max(0, s.queue.length - 1); s.elapsed = 0; }
      else if (wasCurrent) s.elapsed = 0;
      s.history = []; resetShuffle(); notice(wasCurrent ? 'Current entry removed. Advanced if a following track remains.' : 'Queue entry removed.'); break;
    }
    case 'move-up': case 'move-down': {
      const other = i + (act === 'move-up' ? -1 : 1);
      if (other >= 0 && other < s.queue.length) {
        [s.queue[i], s.queue[other]] = [s.queue[other], s.queue[i]];
        if (s.current === i) s.current = other; else if (s.current === other) s.current = i;
        s.history = []; resetShuffle(); notice('Queue reordered. Current track retained.');
      } break;
    }
    case 'depth': s.depth = depth; break;
    case 'show-queue': s.queueOpen = !s.queueOpen; s.searchOpen = false; break;
    case 'open-search': s.queueOpen = false; s.searchOpen = true; render(); $('search')?.focus(); return;
    case 'close-search': s.searchOpen = false; s.query = ''; break;
    case 'clear-search': s.query = ''; break;
    case 'search-artist': selectArtist(artist); s.searchOpen = false; if (s.variant !== 'C') s.query = ''; break;
    case 'search-album': selectAlbum(album); s.searchOpen = false; if (s.variant !== 'C') s.query = ''; break;
    case 'search-track': s.searchContext = 'album'; playAlbum(album, i); s.searchOpen = false; notice(`Playing ${currentTrack().title} within ${label(chosenAlbum())}. Full album queued.`); break;
    case 'help': $('help').showModal(); return;
  }
  render();
}
document.addEventListener('click', event => {
  const button = event.target.closest('[data-act]');
  if (button) handleAction(button);
});
document.addEventListener('dblclick', event => {
  const row = event.target.closest('[data-act="track"]');
  if (row) { playAlbum(row.dataset.album, Number(row.dataset.index)); render(); }
});
document.addEventListener('input', event => {
  if (event.target.id === 'search') { s.query = event.target.value; render(); }
  if (event.target.id === 'seek') { s.elapsed = Number(event.target.value); render(); }
  if (event.target.id === 'volume') { s.volume = Number(event.target.value); render(); }
});
document.addEventListener('keydown', event => {
  if ($('help').open) return;
  const editing = event.target.matches('input,textarea,[contenteditable="true"]');
  if (editing) {
    if (event.key === 'Escape') { event.preventDefault(); s.searchOpen = false; s.query = ''; render(); $('screen').focus(); }
    if (event.key === 'Enter' && event.target.id === 'search') { event.preventDefault(); $('screen').querySelector('[data-nav]')?.focus(); }
    return;
  }
  if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') { event.preventDefault(); cycleVariant(event.key === 'ArrowLeft' ? -1 : 1); return; }
  if (event.key === '/') { event.preventDefault(); s.queueOpen = false; s.searchOpen = true; render(); $('search')?.focus(); return; }
  if (event.key === '?') { event.preventDefault(); $('help').showModal(); return; }
  if (event.key === ' ') { event.preventDefault(); handleAction({dataset:{act:'toggle-play'}}); return; }
  if (event.key === 'a') { event.preventDefault(); handleAction({dataset:{act:'add-album'}}); return; }
  if (event.key === 'Escape') { s.queueOpen = false; s.searchOpen = false; s.query = ''; render(); return; }
  if (event.key === 'Backspace' && s.variant === 'E') { event.preventDefault(); s.depth = s.depth === 'tracks' ? 'albums' : 'artists'; render(); return; }
  if (['j','k','ArrowDown','ArrowUp'].includes(event.key)) {
    const list = event.target.closest('[data-list]') || $('screen').querySelector('[data-list]');
    const rows = [...(list?.querySelectorAll('[data-nav]') || [])];
    if (!rows.length) return;
    event.preventDefault();
    const at = rows.indexOf(document.activeElement);
    const direction = ['j','ArrowDown'].includes(event.key) ? 1 : -1;
    const row = rows[Math.max(0, Math.min(rows.length-1, at < 0 ? 0 : at+direction))];
    row.focus(); row.scrollIntoView({block:'nearest'});
  }
  if (event.key === 'Enter' && event.target.dataset.act === 'track') {
    event.preventDefault(); playAlbum(event.target.dataset.album, Number(event.target.dataset.index)); render();
  }
});
$('previous-variant').onclick = () => cycleVariant(-1);
$('next-variant').onclick = () => cycleVariant(1);
$('theme').onclick = () => { document.body.classList.toggle('light'); };
$('close-help').onclick = () => $('help').close();
window.addEventListener('popstate', () => { const key = new URLSearchParams(location.search).get('variant'); if (variants[key]) { s.variant = key; render(); } });
window.addEventListener('resize', render);
// The clock intentionally stays still until Play is pressed.
setInterval(() => {
  if (!s.playing || !currentTrack() || $('help').open) return;
  s.elapsed++;
  if (s.elapsed >= currentTrack().duration) nextTrack(true);
  const scroll = [...$('screen').querySelectorAll('[data-list],.pane,.shelf,.single,.search-main')].map(el => [el,el.scrollTop]);
  // Only update playback surfaces during the clock tick; keep browser focus and scroll stable.
  const activeId = document.activeElement?.id;
  $('player').innerHTML = renderPlayer();
  if (activeId && $(activeId)) $(activeId).focus({preventScroll:true});
  scroll.forEach(([el,top]) => { el.scrollTop = top; });
  $('state').textContent = JSON.stringify({variant:s.variant, artist:s.artist, album:label(chosenAlbum()), queue:s.queue.map(t=>`${t.artist} — ${t.title}`), currentQueueIndex:s.current, playing:s.playing, elapsedSeconds:s.elapsed, volume:s.volume, shuffle:s.shuffle, repeat:s.repeat, storage:'memory only', audio:'simulated'}, null, 2);
}, 1000);
render();
