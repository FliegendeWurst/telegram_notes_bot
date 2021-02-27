// Create as JS Backend note with attributes:
// ~targetTemplate=@task template (included in Trilium task manager)
// #customRequestHandler=task_alerts
// ~targetTemplateReminder=@reminder template (see new_reminder.js)

const {req, res} = api;

const targetTemplate = await api.currentNote.getRelationValue('targetTemplate');
const tasks = await api.getNotesWithLabel("template", targetTemplate);

let tasksData = [];

for (const task of tasks) {
    tasksData.push({
        attributes: await task.getAttributes(),
        ...task
    });
}

const targetTemplateReminder = await api.currentNote.getRelationValue('targetTemplateReminder');
const reminders = await api.getNotesWithLabel("template", targetTemplateReminder);
for (const task of reminders) {
    tasksData.push({
        attributes: await task.getAttributes(),
        ...task
    });
}

res.send(tasksData);
