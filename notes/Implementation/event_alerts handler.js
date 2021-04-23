const {req, res} = api;

const targetTemplate = await api.currentNote.getRelationValue("targetTemplateEvent");
const events = await api.getNotesWithLabel("template", targetTemplate);

let eventsData = [];

for (const event of events) {
    const attr = await event.getAttribute("label", "startTime");
    eventsData.push({
        name: event.title,
        startTime: attr.value
    });
}

res.send(eventsData);