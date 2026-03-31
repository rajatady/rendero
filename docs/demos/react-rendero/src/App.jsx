// ═══════════════════════════════════════════════════════════════════
// Real JSX on Rendero
//
// JSX rule (same as React):
//   lowercase <frame> → h('frame', ...)    host element (engine node)
//   Uppercase <Button> → h(Button, ...)    your component (function)
//
// So <frame> = engine's Frame node, <text> = engine's Text node
// and <Button>, <Card> etc. are your reusable components.
// ═══════════════════════════════════════════════════════════════════

import { h, useState, mount } from '../renderer.js';

// ─── Components ───

function Button({ title, color = '#007AFF', onPress }) {
    return (
        <frame width={260} height={44} backgroundColor={color} borderRadius={10}
               flexDirection="row" padding={11} onClick={onPress}>
            <text text={title} fontSize={15} color="#ffffff" fontWeight={700} />
        </frame>
    );
}

function TextInput({ value, placeholder, onChange, inputType, secureTextEntry, color = '#1a1a2e' }) {
    return h('TextInput', {
        width: 260, height: 44,
        backgroundColor: '#f0f0f5', borderRadius: 10, padding: 12,
        value: value || '', placeholder: placeholder || '',
        placeholderColor: '#999999', color, fontSize: 15,
        onChange, inputType: inputType || 'text',
        secureTextEntry: secureTextEntry || false,
    });
}

function Label({ text }) {
    return <text text={text} fontSize={12} color="#333" fontWeight={600} />;
}

function Divider() {
    return <frame width={280} height={1} backgroundColor="#e0e0e0" />;
}

function Card({ title, height = 180, children }) {
    return (
        <frame width={310} height={height} backgroundColor="#ffffff"
               borderRadius={16} flexDirection="column" padding={20} gap={10}>
            <text text={title} fontSize={20} color="#1a1a2e" fontWeight={700} />
            {children}
        </frame>
    );
}

function ScrollView({ contentHeight = 900, children }) {
    return (
        <frame x={20} y={50} width={330} height={contentHeight}
               flexDirection="column" gap={14}>
            {children}
        </frame>
    );
}

function GalleryItem({ title, color }) {
    return (
        <frame width={140} height={140} backgroundColor={color} borderRadius={12}>
            <text text={title} fontSize={14} color="#fff" />
        </frame>
    );
}

function Gallery({ items }) {
    return (
        <frame width={300} height={160} flexDirection="row" gap={12}>
            {items.map(item =>
                <GalleryItem title={item.title} color={item.color} />
            )}
        </frame>
    );
}

// ─── Screens ───

function HomeScreen({ navigate }) {
    const [name, setName] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [phone, setPhone] = useState('');
    const [submitted, setSubmitted] = useState(false);
    const photos = [
        { title: 'Sunset', color: '#e74c3c' },
        { title: 'Ocean', color: '#3498db' },
        { title: 'Forest', color: '#27ae60' },
    ];

    const el = document.getElementById('step-label');
    if (el) el.textContent = `name="${name}" email="${email}"`;

    if (submitted) {
        return (
            <ScrollView contentHeight={350}>
                <Gallery items={photos} />
                <Card title="Submitted!" height={220}>
                    <text fontSize={13} color="#555" text={`Name: ${name}`} />
                    <text fontSize={13} color="#555" text={`Email: ${email}`} />
                    <text fontSize={13} color="#555" text={`Phone: ${phone}`} />
                    <text fontSize={13} color="#007AFF" fontWeight={600}
                          text="All state came from controlled inputs." />
                </Card>
                <Button title="Back to Form" onPress={() => setSubmitted(false)} />
            </ScrollView>
        );
    }

    return (
        <ScrollView contentHeight={1000}>
            <Card title="React Native + JSX" height={130}>
                <text fontSize={13} color="#555"
                      text={"Real JSX compiled by esbuild.\nClean syntax, no htm."} />
            </Card>

            <Gallery items={photos} />

            <Card title="Sign Up Form" height={520}>
                <Label text="NAME" />
                <TextInput value={name} placeholder="Enter your name" onChange={setName} />

                <Divider />
                <Label text="EMAIL" />
                <TextInput value={email} placeholder="user@example.com"
                           inputType="email" onChange={setEmail} />

                <Divider />
                <Label text="PASSWORD" />
                <TextInput value={password} placeholder="Secret"
                           secureTextEntry={true} onChange={setPassword} />

                <Divider />
                <Label text="PHONE" />
                <TextInput value={phone} placeholder="+1 (555) 000-0000"
                           inputType="tel" onChange={setPhone} />
            </Card>

            <Button
                title={name ? `Submit as ${name}` : 'Fill in name to submit'}
                color={name ? '#34C759' : '#aaa'}
                onPress={() => { if (name) setSubmitted(true); }}
            />

            <Button title="About" color="#5856D6"
                    onPress={() => navigate('about')} />
        </ScrollView>
    );
}

function AboutScreen({ navigate }) {
    return (
        <ScrollView contentHeight={400}>
            <Card title="How JSX Works" height={280}>
                <text fontSize={13} color="#555"
                      text={"esbuild sees:  <Button title='Go' />\ncompiles to:   h(Button, {title:'Go'})\n\nlowercase = string host element:\n  <frame> → h('frame', ...)\n  <text>  → h('text', ...)\n\nUppercase = your component:\n  <Button> → h(Button, ...)\n  <Card>   → h(Card, ...)"} />
            </Card>
            <Button title="Back" color="#5856D6" onPress={() => navigate('home')} />
        </ScrollView>
    );
}

// ─── Root ───

function App() {
    const [screen, setScreen] = useState('home');
    if (screen === 'about') return <AboutScreen navigate={setScreen} />;
    return <HomeScreen navigate={setScreen} />;
}

mount(App, document.getElementById('canvas'));
