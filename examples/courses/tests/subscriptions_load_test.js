import grpc from 'k6/net/grpc';
import { check, sleep } from 'k6';
import exec from 'k6/execution';

const serverUrl = 'localhost:10437';
const USERS = 500;
const course = {
    course_id: 'course1',
    name: 'Introduction to Programming',
    seats: 100,
};

export let options = {
    vus: USERS,
    duration: '30s',
};

// Create course with 10000 seats
const client = new grpc.Client();
client.load(['../proto'], 'api.proto');

export function setup() {
    client.connect(serverUrl, { plaintext: true });
    client.invoke('api.Course/Create', course);
    // Register students
    for (let i = 1; i <= USERS; i++) {
        const student = {
            student_id: `student${i}`,
            name: `Student ${i}`,
        };
        const res = client.invoke('api.Student/Register', student);
        check(res, { 'registered successfully': (r) => r && r.status === grpc.StatusOK });
    }
    client.close();
}

// Each virtual user subscribes one specific student to the course
export default function (data) {
    client.connect(serverUrl, { plaintext: true });
    const subscription = {
        course_id: course.course_id,
        student_id: `student${exec.vu.idInTest}`,
    };
    const res = client.invoke('api.Subscription/Subscribe', subscription);
    check(res, { 'student subscribed successfully': (r) => r && r.status === grpc.StatusOK });
    check(res, { 'student already subscribed error': (r) => r && r.status === grpc.StatusInternal && r.error.message === 'student already subscribed' });
    check(res, { 'course no seats available': (r) => r && r.status === grpc.StatusInternal && r.error.message === 'no seats available' });
    check(res, { 'concurrent modification error': (r) => r && r.status === grpc.StatusInternal && r.error.message === 'concurrent modification error' });
    client.close();
}