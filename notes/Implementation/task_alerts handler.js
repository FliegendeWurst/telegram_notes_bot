const {req, res} = api;

const today = new Date().toISOString().substr(0, 10);

const targetTemplate = await api.currentNote.getRelationValue('targetTemplate');
const tasks = await api.getNotesWithLabel("template", targetTemplate);

let tasksData = [];

for (const task of tasks) {
    const todoDate = task.getAttribute("label", "todoDate");
    if (!todoDate || todoDate["value"] < today) {
        continue;
    }
    tasksData.push({
        attributes: await task.getAttributes(),
        ...task
    });
}

const targetTemplateReminder = await api.currentNote.getRelationValue('targetTemplateReminder');
const reminders = await api.getNotesWithLabel("template", targetTemplateReminder);
for (const task of reminders) {
    const todoDate = task.getAttribute("label", "todoDate");
    if (!todoDate || todoDate["value"] < today) {
        continue;
    }
    tasksData.push({
        attributes: await task.getAttributes(),
        ...task
    });
}

const targetTemplateReminderDaily = await api.currentNote.getRelationValue('targetTemplateReminderDaily');
const remindersDaily = await api.getNotesWithLabel("template", targetTemplateReminderDaily);
for (const task of remindersDaily) {
    const attributes = await task.getAttributes();
    attributes.push({
        "type": "label",
        "name": "todoDate",
        "value": today,
        // API compliance
        "attributeId": "",
        "noteId": "",
        "position": 0,
        "utcDateModified": "2021-04-23 08:20:14.295Z",
        "isDeleted": false,
        "isInheritable": false
    });
    tasksData.push({
        attributes: attributes,
        ...task
    });
}

res.send(tasksData);
