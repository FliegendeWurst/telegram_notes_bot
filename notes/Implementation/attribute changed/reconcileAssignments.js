module.exports = async function(note, categoryRootNote, assignedCategories, labelName, isTaskDone) {
    const found = {};
    
    for (const categoryNote of await categoryRootNote.getChildNotes()) {
        const label = await categoryNote.getLabel(labelName);
        
        if (label) {
            found[label.value] = !isTaskDone && assignedCategories.includes(label.value);

            await api.toggleNoteInParent(found[label.value], note.noteId, categoryNote.noteId);
        }
    }
    
    if (!isTaskDone) {
        for (const assignedCategory of assignedCategories) {
            if (!found[assignedCategory]) {
                const categoryNote = (await api.createNote(categoryRootNote.noteId, assignedCategory, "", {
                    attributes: [ { type: "label", name: labelName, value: assignedCategory } ]
                })).note;

                await api.ensureNoteIsPresentInParent(note.noteId, categoryNote.noteId);
            }
        }
    }
}