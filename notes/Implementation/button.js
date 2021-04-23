api.addButtonToToolbar({
    title: 'New task',
    icon: 'check',
    shortcut: 'alt+n',
    action: async () => {
        // creating notes is backend (server) responsibility so we need to pass
        // the control there
        const taskNoteId = await api.runOnServer(async () => {
            const todoRootNote = await api.getNoteWithLabel('taskTodoRoot');
            const resp = await api.createTextNote(todoRootNote.noteId, 'new task', '');

            return resp.note.noteId;
        });

        await api.waitUntilSynced();
        // we got an ID of newly created note and we want to immediatelly display it
        await api.activateNewNote(taskNoteId);
    }
});