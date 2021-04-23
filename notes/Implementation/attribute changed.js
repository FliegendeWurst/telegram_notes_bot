if (!["task", "location", "tag", "todoDate", "doneDate"].includes(api.originEntity.name)) {
    return;
}

const note = await api.originEntity.getNote();
    
const attributes = await note.getAttributes();

if (attributes.filter(attr => attr.type === 'label' && attr.name === 'reminder').map(attr => attr.value).length() > 0) {
    api.log("ignoring");
    return;
}

const todoDate = await note.getLabelValue('todoDate');
const doneDate = await note.getLabelValue('doneDate');
const canceled = !!(await note.getLabelValue('canceled'));
const isTaskDone = !!doneDate;

const canceledRootNote = await api.getNoteWithLabel('taskDoneRoot');
await api.toggleNoteInParent(canceled, note.noteId, canceledRootNote.noteId);

const doneRootNote = await api.getNoteWithLabel('taskDoneRoot');
await api.toggleNoteInParent(isTaskDone && !canceled, note.noteId, doneRootNote.noteId);

const todoRootNote = await api.getNoteWithLabel('taskTodoRoot');
await api.toggleNoteInParent(!isTaskDone && !canceled, note.noteId, todoRootNote.noteId);

const location = await note.getLabelValue('location');
const locationRootNote = await api.getNoteWithLabel('taskLocationRoot');

await reconcileAssignments(note, locationRootNote, location ? [location] : [], 'taskLocationNote', isTaskDone);

const tags = attributes.filter(attr => attr.type === 'label' && attr.name === 'tag').map(attr => attr.value);
const tagRootNote = await api.getNoteWithLabel('taskTagRoot');

await reconcileAssignments(note, tagRootNote, tags, 'taskTagNote', isTaskDone);

await note.toggleLabel(isTaskDone || canceled, "cssClass", "done");

const doneTargetNoteId = isTaskDone ? (await api.getDateNote(doneDate)).noteId : null;
await api.setNoteToParent(note.noteId, 'DONE', doneTargetNoteId);

await note.toggleLabel(!isTaskDone && !canceled, "cssClass", "todo");

const todoTargetNoteId = (!isTaskDone && !canceled && todoDate) ? (await api.getDateNote(todoDate)).noteId : null;
await api.setNoteToParent(note.noteId, 'TODO', todoTargetNoteId);

api.refreshTree();