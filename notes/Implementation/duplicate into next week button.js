api.addButtonToToolbar({
    title: 'Duplicate into next week',
    icon: 'right-arrow-alt',
    action: async () => {
        await api.runOnBackend(async (noteId) => {
const note = await api.getNote(noteId);
const targetTemplate = await note.getRelation('template').value;
const startTime = await note.getLabelValue('startTime');
const location = await note.getLabelValue('location');
const date = new Date(startTime);
date.setTime(date.getTime() + (7*24*60*60*1000));

const todayDateStr = date.toISOString().substr(0,10);
const todayNote = await api.getDateNote(todayDateStr);
const newTime = todayDateStr + "T" + date.getHours().toString().padStart(2, '0') + ":" + date.getMinutes().toString().padStart(2, '0') + ":" + date.getSeconds().toString().padStart(2, '0');

const resp = await api.createNewNote({
    parentNoteId: todayNote.noteId,
    title: note.title,
    content: '',
    type: 'text'
});
await resp.note.setAttribute("relation", "template", targetTemplate);
await resp.note.setAttribute("label", "startTime", newTime);
if (location) {
    await resp.note.setAttribute("label", "location", location);
}
}, [api.getActiveTabNote().noteId]);
    }
});