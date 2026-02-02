import grpc from 'k6/net/grpc';
import { check, sleep } from 'k6';

const serverUrl = 'localhost:10437';
const USERS = 500;
const COURSES = 10;
const STUDENTS = 500;

const COURSE_ID_PREFIX = 'course';
const STUDENT_ID_PREFIX = 'student';

export let options = {
    vus: USERS,
    duration: '60s',
};

const client = new grpc.Client();
client.load(['../proto'], 'api.proto');

export function setup() {
    // Create courses
    client.connect(serverUrl, { plaintext: true });
    for (let i = 1; i <= COURSES; i++) {
        const course = {
            course_id: `${COURSE_ID_PREFIX}${i}`,
            name: 'Introduction to Programming',
            seats: 100,
        };
        const res = client.invoke('api.Course/Create', course);
        check(res, { 'course created': (r) => r && r.status === grpc.StatusOK });
    }
    // Register students
    for (let i = 1; i <= STUDENTS; i++) {
        const student = {
            student_id: `${STUDENT_ID_PREFIX}${i}`,
            name: `Student ${i}`,
        };
        const res = client.invoke('api.Student/Register', student);
        check(res, { 'registered successfully': (r) => r && r.status === grpc.StatusOK });
    }
    client.close();
}

// Each virtual user subscribes one specific student to the course
export default function(data) {
    client.connect(serverUrl, { plaintext: true });
    const course_id = Math.floor(Math.random() * COURSES) + 1;
    const student_id = Math.floor(Math.random() * STUDENTS) + 1;
    const subscription = {
        course_id: `${COURSE_ID_PREFIX}${course_id}`,
        student_id: `${STUDENT_ID_PREFIX}${student_id}`,
    };
    const res = client.invoke('api.Subscription/Subscribe', subscription);
    check(res, { 'student subscribed successfully': (r) => r && r.status === grpc.StatusOK });
    check(res, { 'student already subscribed error': (r) => r && r.status === grpc.StatusInternal && r.error.message.includes('student already subscribed') });
    check(res, { 'student has too many courses error': (r) => r && r.status === grpc.StatusInternal && r.error.message.includes('student has too many courses') });
    check(res, { 'course no seats available error': (r) => r && r.status === grpc.StatusInternal && r.error.message.includes('no seats available') });
    check(res, { 'concurrent modification error': (r) => r && r.status === grpc.StatusInternal && r.error.message.includes('concurrent modification error') });
    client.close();
}
