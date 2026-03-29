// ═══════════════════════════════════════════════════════════════════
// React Native on Rendero — ScrollView + TextInput
// ═══════════════════════════════════════════════════════════════════

import { h, useState, mount } from './renderer.js';
import htm from 'https://esm.sh/htm@3.1.1';

const html = htm.bind(h);

// ─── StyleSheet ───

const styles = {
    card: {
        width: 310, backgroundColor: '#ffffff', borderRadius: 16,
        flexDirection: 'column', padding: 20, gap: 10,
    },
    title: { fontSize: 20, color: '#1a1a2e', fontWeight: 700 },
    body: { fontSize: 13, color: '#555555' },
    accent: { fontSize: 13, color: '#007AFF', fontWeight: 600 },
};

// ─── Built-in Components ───

function Button({ title, color = '#007AFF', onPress }) {
    return html`
        <Frame width=260 height=44 backgroundColor=${color} borderRadius=10
               flexDirection="row" padding=11 onClick=${onPress}>
            <Text text=${title} fontSize=15 color="#ffffff" fontWeight=700 />
        </Frame>
    `;
}

function TextInput({ value, placeholder, onChange, onSubmit, inputType, secureTextEntry, color = '#1a1a2e' }) {
    return h('TextInput', {
        width: 260, height: 44,
        backgroundColor: '#f0f0f5',
        borderRadius: 10,
        padding: 12,
        value: value || '',
        placeholder: placeholder || '',
        placeholderColor: '#999999',
        color,
        fontSize: 15,
        onChange, onSubmit,
        inputType: inputType || 'text',
        secureTextEntry: secureTextEntry || false,
    });
}

function Label({ text }) {
    return html`<Text text=${text} fontSize=12 color="#333" fontWeight=600 />`;
}

function Divider() {
    return html`<Frame width=280 height=1 backgroundColor="#e0e0e0" />`;
}

// ─── ScrollView ───
// Wraps content in a tall Frame. The camera-based scrolling handles the rest.
// In React Native, ScrollView is a native component that clips + handles gestures.
// Here we set the scrollable height based on total content.

function ScrollView({ contentHeight = 900, children }) {
    return html`
        <Frame x=20 y=50 width=330 height=${contentHeight}
               flexDirection="column" gap=14>
            ${children}
        </Frame>
    `;
}

// ─── Screens ───

function HomeScreen({ navigate }) {
    const [name, setName] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [phone, setPhone] = useState('');
    const [submitted, setSubmitted] = useState(false);

    const el = document.getElementById('step-label');
    if (el) el.textContent = `name="${name}" email="${email}"`;

    if (submitted) {
        return html`
            <${ScrollView} contentHeight=350>
                <Frame ...${styles.card} height=220>
                    <Text ...${styles.title} text="Submitted!" />
                    <Text ...${styles.body} text="Name: ${name}" />
                    <Text ...${styles.body} text="Email: ${email}" />
                    <Text ...${styles.body} text="Phone: ${phone}" />
                    <Text ...${styles.accent}
                        text="All state came from controlled\nTextInputs via useState." />
                </Frame>
                <${Button} title="Back to Form"
                           onPress=${() => setSubmitted(false)} />
            <//>
        `;
    }

    return html`
        <${ScrollView} contentHeight=900>
            <Frame ...${styles.card} height=130>
                <Text ...${styles.title} text="React Native Inputs" />
                <Text ...${styles.body}
                    text="Tap a field to type. Scroll to see\nmore. Each input is controlled." />
            </Frame>

            <Frame ...${styles.card} height=520>
                <Text ...${styles.title} text="Sign Up Form" />

                <${Label} text="NAME" />
                <${TextInput} value=${name} placeholder="Enter your name"
                              onChange=${setName} />

                <${Divider} />
                <${Label} text="EMAIL" />
                <${TextInput} value=${email} placeholder="user@example.com"
                              inputType="email" onChange=${setEmail} />

                <${Divider} />
                <${Label} text="PASSWORD" />
                <${TextInput} value=${password} placeholder="Secret"
                              secureTextEntry=${true} onChange=${setPassword} />

                <${Divider} />
                <${Label} text="PHONE" />
                <${TextInput} value=${phone} placeholder="+1 (555) 000-0000"
                              inputType="tel" onChange=${setPhone} />
            </Frame>

            <${Button}
                title=${name ? `Submit as ${name}` : 'Fill in name to submit'}
                color=${name ? '#34C759' : '#aaa'}
                onPress=${() => { if (name) setSubmitted(true); }} />

            <${Button} title="About this demo" color="#5856D6"
                       onPress=${() => navigate('about')} />
        <//>
    `;
}

function AboutScreen({ navigate }) {
    return html`
        <${ScrollView} contentHeight=400>
            <Frame ...${styles.card} height=250>
                <Text ...${styles.title} text="How It Works" />
                <Text ...${styles.body}
                    text="1. You write components (app.js)\n2. htm parses JSX-like templates\n3. Reconciler diffs virtual trees\n4. Only changed nodes update\n5. Rendero engine draws pixels\n6. Hidden HTML inputs capture keyboard\n7. Camera offset handles scrolling" />
            </Frame>
            <${Button} title="Back" color="#5856D6"
                       onPress=${() => navigate('home')} />
        <//>
    `;
}

// ─── Root ───

function App() {
    const [screen, setScreen] = useState('home');

    if (screen === 'about') {
        return html`<${AboutScreen} navigate=${setScreen} />`;
    }
    return html`<${HomeScreen} navigate=${setScreen} />`;
}

mount(App, document.getElementById('canvas'));
